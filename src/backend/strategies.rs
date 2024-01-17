//! Strategies management backend module.

use std::{collections::{BTreeMap, BTreeSet, HashMap, VecDeque}, sync::Arc};

use dapi_grpc::platform::v0::{GetEpochsInfoRequest, get_epochs_info_request, get_epochs_info_response, wait_for_state_transition_result_response};
use dash_platform_sdk::{Sdk, platform::transition::broadcast_request::BroadcastRequestForStateTransition};
use dpp::{
    data_contract::created_data_contract::CreatedDataContract, platform_value::{Bytes32, Identifier},
    version::PlatformVersion, block::{block_info::BlockInfo, epoch::Epoch}, identity::{Identity, PartialIdentity, state_transition::asset_lock_proof::AssetLockProof, accessors::IdentityGettersV0}, state_transition::StateTransition, dashcore::PrivateKey,
};
use drive::{drive::{identity::key::fetch::IdentityKeysRequest, document::query::{QueryDocumentsOutcome, QueryDocumentsOutcomeV0Methods}}, query::DriveQuery};
use futures::future::join_all;
use rand::{rngs::StdRng, SeedableRng};
use rs_dapi_client::{RequestSettings, DapiRequest};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{
    frequency::Frequency, operations::{Operation, FinalizeBlockOperation}, transitions::create_identities_state_transitions,
    Strategy, LocalDocumentQuery, StrategyConfig,
};
use tokio::sync::{MutexGuard, Mutex};

use tracing::{info, error};

use rs_dapi_client::DapiRequestExecutor;



use crate::backend::Wallet;

use super::{AppStateUpdate, BackendEvent, AppState, StrategyCompletionResult, error::Error, insight::{InsightError, InsightAPIClient}};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum StrategyTask {
    CreateStrategy(String),
    SelectStrategy(String),
    DeleteStrategy(String),
    CloneStrategy(String),
    SetContractsWithUpdates(String, Vec<String>),
    SetIdentityInserts {
        strategy_name: String,
        identity_inserts_frequency: Frequency,
    },
    // SetStartIdentities {
    //     strategy_name: String,
    //     count: u16,
    //     key_count: u32,
    // },
    AddOperation {
        strategy_name: String,
        operation: Operation,
    },
    RunStrategy(String),
    RemoveLastContract(String),
    RemoveIdentityInserts(String),
    RemoveStartIdentities(String),
    RemoveLastOperation(String),
}

pub(crate) async fn run_strategy_task<'s>(
    sdk: Arc<Sdk>,
    app_state: &'s AppState,
    task: StrategyTask,
    insight: &'s InsightAPIClient,
) -> BackendEvent<'s> {
    match task {
        StrategyTask::CreateStrategy(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;

            strategies_lock.insert(
                strategy_name.clone(),
                Strategy {
                    contracts_with_updates: Default::default(),
                    operations: Default::default(),
                    start_identities: Default::default(),
                    identities_inserts: Default::default(),
                    signer: Default::default(),
                },
            );
            contract_names_lock.insert(strategy_name, Default::default());
            BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                strategies_lock,
                contract_names_lock,
            ))
        }
        StrategyTask::SelectStrategy(ref strategy_name) => {
            let mut selected_strategy_lock = app_state.selected_strategy.lock().await;
            let strategies_lock = app_state.available_strategies.lock().await;

            if strategies_lock.contains_key(strategy_name) {
                *selected_strategy_lock = Some(strategy_name.clone());
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::DeleteStrategy(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;
            let mut selected_strategy_lock = app_state.selected_strategy.lock().await;

            // Check if the strategy exists and remove it
            if strategies_lock.contains_key(&strategy_name) {
                strategies_lock.remove(&strategy_name);
                contract_names_lock.remove(&strategy_name);

                // If the deleted strategy was the selected one, unset the selected strategy
                if let Some(selected) = selected_strategy_lock.as_ref() {
                    if selected == &strategy_name {
                        *selected_strategy_lock = None;
                    }
                }

                BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                    strategies_lock,
                    contract_names_lock,
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::CloneStrategy(new_strategy_name) => {
            let strategies_lock = app_state.available_strategies.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;
            let selected_strategy_lock = app_state.selected_strategy.lock().await;

            if let Some(selected_strategy_name) = &*selected_strategy_lock {
                if let Some(strategy_to_clone) = strategies_lock.get(selected_strategy_name) {
                    let cloned_strategy = strategy_to_clone.clone();
                    drop(strategies_lock); // Release the lock before re-acquiring it as mutable

                    // Clone the display data for the new strategy
                    let cloned_display_data = contract_names_lock
                        .get(selected_strategy_name)
                        .cloned()
                        .unwrap_or_default();

                    let mut strategies_lock = app_state.available_strategies.lock().await;
                    strategies_lock.insert(new_strategy_name.clone(), cloned_strategy);
                    contract_names_lock.insert(new_strategy_name.clone(), cloned_display_data);

                    BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                        strategies_lock,
                        contract_names_lock,
                    ))
                } else {
                    BackendEvent::None // Selected strategy does not exist
                }
            } else {
                BackendEvent::None // No strategy selected to clone
            }
        }
        StrategyTask::SetContractsWithUpdates(strategy_name, selected_contract_names) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let known_contracts_lock = app_state.known_contracts.lock().await;
            let supporting_contracts_lock = app_state.supporting_contracts.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;
            
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                let mut rng = StdRng::from_entropy();
                let platform_version = PlatformVersion::latest();
        
                // Function to retrieve the contract from either known_contracts or supporting_contracts
                let get_contract = |contract_name: &String| {
                    known_contracts_lock.get(contract_name)
                        .or_else(|| supporting_contracts_lock.get(contract_name))
                        .cloned()
                };
        
                if let Some(first_contract_name) = selected_contract_names.first() {
                    if let Some(data_contract) = get_contract(first_contract_name) {

                        let entropy = Bytes32::random_with_rng(&mut rng);
                        match CreatedDataContract::from_contract_and_entropy(
                            data_contract,
                            entropy,
                            platform_version,
                        ) {
                            Ok(initial_contract) => {
                                let mut updates = BTreeMap::new();
        
                                for (order, contract_name) in selected_contract_names.iter().enumerate().skip(1) {
                                    if let Some(update_contract) = get_contract(contract_name) {
                                        let update_entropy = Bytes32::random_with_rng(&mut rng);
                                        match CreatedDataContract::from_contract_and_entropy(
                                            update_contract,
                                            update_entropy,
                                            platform_version,
                                        ) {
                                            Ok(created_update_contract) => {
                                                updates.insert(order as u64, created_update_contract);
                                            }
                                            Err(e) => {
                                                error!("Error converting DataContract to CreatedDataContract for update: {:?}", e);
                                            }
                                        }
                                    }
                                }
        
                                strategy.contracts_with_updates.push((
                                    initial_contract,
                                    if updates.is_empty() { None } else { Some(updates) },
                                ));
                            }
                            Err(e) => {
                                error!("Error converting DataContract to CreatedDataContract: {:?}", e);
                            }
                        }
                    }
                }
        
                let mut transformed_contract_names = Vec::new();
                if let Some(first_contract_name) = selected_contract_names.first() {
                    let updates: BTreeMap<u64, String> = selected_contract_names.iter().enumerate().skip(1)
                        .map(|(order, name)| (order as u64, name.clone()))
                        .collect();
                    transformed_contract_names.push((first_contract_name.clone(), Some(updates)));
                }
        
                if let Some(existing_contracts) = contract_names_lock.get_mut(&strategy_name) {
                    existing_contracts.extend(transformed_contract_names);
                } else {
                    contract_names_lock.insert(strategy_name.clone(), transformed_contract_names);
                }
        
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(contract_names_lock, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::AddOperation {
            ref strategy_name,
            ref operation,
        } => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                strategy.operations.push(operation.clone());
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::SetIdentityInserts {
            strategy_name,
            identity_inserts_frequency,
        } => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                strategy.identities_inserts = identity_inserts_frequency;
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        // StrategyTask::SetStartIdentities {
        //     ref strategy_name,
        //     count,
        //     key_count,
        // } => {
        //     let mut strategies_lock = app_state.available_strategies.lock().await;
        //     let loaded_identity_lock = app_state.loaded_identity.lock().await;
        //     let identity_private_keys_lock = app_state.identity_private_keys.lock().await;
        
        //     if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
        //         // Ensure a signer is present, creating a new one if necessary
        //         let signer = if let Some(signer) = strategy.signer.as_mut() {
        //             // Use the existing signer
        //             signer
        //         } else {
        //             // Create a new signer from loaded_identity if one doesn't exist, else default
        //             let new_signer = if let Some(loaded_identity) = &*loaded_identity_lock {
        //                 let mut signer = SimpleSigner::default();
        //                 match loaded_identity {
        //                     Identity::V0(identity_v0) => {
        //                         for (key_id, public_key) in &identity_v0.public_keys {
        //                             let identity_key_tuple = (identity_v0.id, *key_id);
        //                             if let Some(private_key_bytes) = identity_private_keys_lock.get(&identity_key_tuple) {
        //                                 signer.private_keys.insert(public_key.clone(), private_key_bytes.to_bytes());
        //                             }
        //                         }
        //                     }
        //                 }
        //                 signer
        //             } else {
        //                 SimpleSigner::default()
        //             };
        //             strategy.signer = Some(new_signer);
        //             strategy.signer.as_mut().unwrap()
        //         };
                                
        //         // Call set_start_identities asynchronously
        //         match set_start_identities(count, key_count, signer, app_state, &sdk).await {
        //             Ok(identities_and_transitions) => {
        //                 strategy.start_identities = identities_and_transitions;
        //                 BackendEvent::TaskCompletedStateChange {
        //                     task: Task::Strategy(task.clone()),
        //                     execution_result: Ok("Start identities set".into()),
        //                     app_state_update: AppStateUpdate::SelectedStrategy(
        //                         strategy_name.to_string(),
        //                         MutexGuard::map(strategies_lock, |strategies| {
        //                             strategies.get_mut(strategy_name).expect("strategy exists")
        //                         }),
        //                         MutexGuard::map(
        //                             app_state.available_strategies_contract_names.lock().await,
        //                             |names| names.get_mut(strategy_name).expect("inconsistent data"),
        //                         ),
        //                     ),
        //                 }
        //             },
        //             Err(e) => {
        //                 eprintln!("Error setting start identities: {:?}", e);
        //                 BackendEvent::StrategyError {
        //                     strategy_name: strategy_name.clone(),
        //                     error: format!("Error setting start identities: {}", e),
        //                 }
        //             }
        //         }
        //     } else {
        //         BackendEvent::None
        //     }
        // }
        StrategyTask::RunStrategy(strategy_name) => {
            info!("Starting strategy '{}'", strategy_name);

            let mut strategies_lock = app_state.available_strategies.lock().await;
            let drive_lock = app_state.drive.lock().await;
            let mut loaded_wallet_lock = app_state.loaded_wallet.lock().await;
            let identity_private_keys_lock = app_state.identity_private_keys.lock().await;
            let mut loaded_identity_lock = match app_state.refresh_identity(&sdk).await {
                Ok(lock) => lock,
                Err(e) => {
                    // Handle the error, for example, log it and return an error event
                    error!("Failed to refresh identity: {:?}", e);
                    return BackendEvent::StrategyError {
                        strategy_name: strategy_name.clone(),
                        error: format!("Failed to refresh identity: {:?}", e),
                    };
                }
            };

            // Check if a loaded identity is available and clone it for use in the loop
            let mut loaded_identity_clone = loaded_identity_lock.clone();
                        
            // It's normal that we're asking for the mutable strategy because we need to modify
            // some properties of a contract on update
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                // Define the number of blocks for which to compute state transitions.
                let num_blocks = 20;
            
                // Get block_info
                // Get block info for the first block by sending a grpc request and looking at the metadata
                // Retry up to MAX_RETRIES times
                const MAX_RETRIES: u8 = 2;
                let mut initial_block_info = BlockInfo::default();
                let mut retries = 0;
                let request = GetEpochsInfoRequest {
                    version: Some(get_epochs_info_request::Version::V0(
                        get_epochs_info_request::GetEpochsInfoRequestV0 {
                            start_epoch: None,
                            count: 1,
                            ascending: false,
                            prove: false,
                        },
                    )),
                };
                while retries <= MAX_RETRIES {
                    match sdk.execute(request.clone(), RequestSettings::default()).await {
                        Ok(response) => {
                            if let Some(get_epochs_info_response::Version::V0(response_v0)) = response.version {
                                if let Some(metadata) = response_v0.metadata {
                                    initial_block_info = BlockInfo {
                                        time_ms: metadata.time_ms,
                                        height: metadata.height,
                                        core_height: metadata.core_chain_locked_height,
                                        epoch: Epoch::new(metadata.epoch as u16).unwrap(),
                                    };
                                }
                            }
                            break;
                        },
                        Err(e) if retries < MAX_RETRIES => {
                            error!("Error executing request, retrying: {:?}", e);
                            retries += 1;
                        },
                        Err(e) => {
                            error!("Failed to execute request after retries: {:?}", e);
                            return BackendEvent::StrategyError {
                                strategy_name: strategy_name.clone(),
                                error: format!("Failed to execute request after retries: {:?}", e),
                            };
                        }
                    }
                }
                initial_block_info.height += 1; // Add one because we'll be submitting to the next block

                // Get signer from loaded_identity
                // Convert loaded_identity to SimpleSigner
                let mut signer = {
                    let strategy_signer = strategy.signer.get_or_insert_with(|| {
                        let mut new_signer = SimpleSigner::default();
                        let Identity::V0(identity_v0) = &*loaded_identity_lock;
                        for (key_id, public_key) in &identity_v0.public_keys {
                            let identity_key_tuple = (identity_v0.id, *key_id);
                            if let Some(private_key_bytes) = identity_private_keys_lock.get(&identity_key_tuple) {
                                new_signer.private_keys.insert(public_key.clone(), private_key_bytes.to_bytes());
                            }
                        }
                        new_signer
                    });
                    strategy_signer.clone()
                };
                                
                // Check if a loaded wallet is available and it is mutable
                let wallet = match loaded_wallet_lock.as_mut() {
                    Some(w) => w,
                    None => {
                        error!("No wallet loaded");
                        return BackendEvent::StrategyError {
                            strategy_name: strategy_name.clone(),
                            error: "No wallet loaded".to_string(),
                        };
                    }
                };
        
                // Create map of state transitions to block heights
                let mut state_transitions_map = HashMap::new();

                // Copy initial block info
                let mut current_block_info = initial_block_info.clone();

                // Fill out the state transitions map
                // Loop through each block to precompute state transitions
                while current_block_info.height < (initial_block_info.height + num_blocks) {
                    let mut document_query_callback = |query: LocalDocumentQuery| {
                        match query {
                            LocalDocumentQuery::RandomDocumentQuery(random_query) => {
                                let document_type = random_query.document_type;
                                let data_contract = random_query.data_contract;
                            
                                // Construct a DriveQuery based on the document_type and data_contract
                                let drive_query = DriveQuery::any_item_query(data_contract, document_type.as_ref());
                            
                                // Query the Drive for documents
                                match drive_lock.query_documents(drive_query, None, false, None, None) {
                                    Ok(outcome) => {
                                        match outcome {
                                            QueryDocumentsOutcome::V0(outcome_v0) => {
                                                let documents = outcome_v0.documents_owned();
                                                info!("Fetched {} documents using DriveQuery", documents.len());
                                                documents
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        error!("Block {}: Error fetching documents using DriveQuery: {:?}", current_block_info.height, e);
                                        vec![]
                                    }
                                }
                            }
                        }
                    };
                    let mut identity_fetch_callback = |identifier: Identifier, _keys_request: Option<IdentityKeysRequest>| {
                        // Convert Identifier to a byte array format expected by the Drive method
                        let identity_id_bytes = identifier.into_buffer();
                    
                        // Fetch identity information from the Drive
                        match drive_lock.fetch_identity_with_balance(
                            identity_id_bytes,
                            None,
                            PlatformVersion::latest()
                        ) {
                            Ok(maybe_partial_identity) => {
                                let partial_identity = maybe_partial_identity.unwrap_or_else(|| PartialIdentity {
                                    id: identifier,
                                    loaded_public_keys: BTreeMap::new(),
                                    balance: None,
                                    revision: None,
                                    not_found_public_keys: BTreeSet::new(),
                                });
                                info!("Fetched identity info for identifier {}: {:?}", identifier, partial_identity);
                                partial_identity
                            },
                            Err(e) => {
                                error!("Error fetching identity: {:?}", e);
                                PartialIdentity {
                                    id: identifier,
                                    loaded_public_keys: BTreeMap::new(),
                                    balance: None,
                                    revision: None,
                                    not_found_public_keys: BTreeSet::new(),
                                }
                            }
                        }
                    };

                    let sdk_ref = &sdk;
                    let wallet_clone = wallet.clone();

                    let mut create_asset_lock = move |amount: u64| -> Option<(AssetLockProof, PrivateKey)> {
                        let mut wallet_clone = wallet_clone.clone();
                    
                        let future = async move {
                            let (asset_lock_transaction, asset_lock_proof_private_key) = match wallet_clone.asset_lock_transaction(None, amount) {
                                Ok(result) => result,
                                Err(_) => return None,
                            };
                    
                            match AppState::broadcast_and_retrieve_asset_lock(sdk_ref, &asset_lock_transaction, &wallet_clone.receive_address()).await {
                                Ok(proof) => Some((proof, asset_lock_proof_private_key)),
                                Err(_) => None,
                            }
                        };
                    
                        tokio::task::block_in_place(|| {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(future)
                        })
                    };
                                                                                                                                                                                                                            
                    // Get current identities
                    let mut current_identities: Vec<Identity> = vec![loaded_identity_clone.clone()];
                                    
                    // Get rng
                    let mut rng = StdRng::from_entropy();

                    // Call the function to get STs for block
                    let (transitions, finalize_operations) = strategy
                        .state_transitions_for_block_with_new_identities(
                            &mut document_query_callback,
                            &mut identity_fetch_callback,
                            &mut create_asset_lock,
                            &current_block_info,
                            &mut current_identities,
                            &mut signer,
                            &mut rng,
                            &StrategyConfig { start_block_height: initial_block_info.height },
                            PlatformVersion::latest(),
                        )
                        .await;

                    // TO-DO: add documents from state transitions to explorer.drive here
                    // this is required for DocumentDelete and DocumentReplace strategy operations
                                                
                    // Process each FinalizeBlockOperation
                    for operation in finalize_operations {
                        match operation {
                            FinalizeBlockOperation::IdentityAddKeys(identifier, keys) => {
                                if let Some(identity) = current_identities.iter_mut().find(|id| id.id() == identifier) {
                                    for key in keys {
                                        identity.add_public_key(key);
                                    }
                                }
                            }
                        }
                    }
                
                    // Update the loaded_identity_clone and loaded_identity_lock with the latest state of the identity
                    if let Some(modified_identity) = current_identities.get(0) {
                        loaded_identity_clone = modified_identity.clone();
                        *loaded_identity_lock = modified_identity.clone();
                    }
                                                        
                    // Store the state transitions in the map if not empty
                    if !transitions.is_empty() {
                        let st_queue = VecDeque::from(transitions.clone());
                        state_transitions_map.insert(current_block_info.height, st_queue.clone());
                        info!("Prepared {} state transitions for block {}", st_queue.len(), current_block_info.height);
                    } else {
                        // Log when no state transitions are found for a block
                        info!("No state transitions prepared for block {}", current_block_info.height);
                    }

                    // Reload wallet UTXOs if an IdentityCreate transition was created
                    let mut identity_created = false;
                    for transition in &transitions {
                        if let StateTransition::IdentityCreate(_) = transition {
                            identity_created = true;
                            break;
                        }
                    }
                    if identity_created {
                        let max_retries = 5; // Maximum number of retries
                        let mut retries = 0;
                    
                        // Initialize old_utxos outside the loop
                        let old_utxos = match wallet {
                            Wallet::SingleKeyWallet(ref wallet) => wallet.utxos.clone(),
                            // Add handling for other wallet types if needed
                        };
                    
                        while retries < max_retries {
                            wallet.reload_utxos(&insight).await;
                    
                            // Check if new UTXOs are available
                            let found_new_utxos = match wallet {
                                Wallet::SingleKeyWallet(ref wallet) => wallet.utxos != old_utxos,
                                // Add handling for other wallet types if needed
                            };
                    
                            if found_new_utxos {
                                info!("New UTXOs found, proceeding with transactions.");
                                break;
                            } else {
                                tracing::warn!("No new UTXOs found, retrying reload (attempt {}/{})", retries + 1, max_retries);
                                retries += 1;
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Sleep before retrying
                            }
                        }
                    
                        if retries == max_retries {
                            error!("Failed to find new UTXOs after reloading, aborting operation.");
                            return BackendEvent::StrategyError { 
                                strategy_name: strategy_name.clone(), 
                                error: "Failed to find new UTXOs after maximum retries".to_string(),
                            };
                        }
                    }
                                                                                                                                                                                            
                    // Update block_info
                    current_block_info.height += 1;
                    current_block_info.time_ms += 1 * 1000; // plus 1 second
                }
            
                let mut current_block_height = initial_block_info.height;

                info!("--- Starting strategy execution loop ---");

                // Iterate over each block height
                while current_block_height < (initial_block_info.height + num_blocks) {                
                    if let Some(transitions) = state_transitions_map.get(&current_block_height) {
                        let mut broadcast_futures = Vec::new();
                        let mut transition_type = String::new();

                        for transition in transitions {
                            let sdk_clone = Arc::clone(&sdk);
                            let transition_clone = transition.clone();

                            transition_type = match transition_clone {
                                StateTransition::DataContractCreate(_) => "DataContractCreate".to_string(),
                                StateTransition::DataContractUpdate(_) => "DataContractUpdate".to_string(),
                                StateTransition::DocumentsBatch(_) =>  "DocumentsBatch".to_string(),
                                StateTransition::IdentityCreate(_) => "IdentityCreate".to_string(),
                                StateTransition::IdentityTopUp(_) => "IdentityTopUp".to_string(),
                                StateTransition::IdentityCreditWithdrawal(_) => "IdentityCreditWithdrawal".to_string(),
                                StateTransition::IdentityUpdate(_) => "IdentityUpdate".to_string(),
                                StateTransition::IdentityCreditTransfer(_) => "IdentityCreditTransfer".to_string(),
                            };
                
                            // Collect futures for broadcasting state transitions
                            let future = async move {
                                match transition_clone.broadcast_request_for_state_transition() {
                                    Ok(broadcast_request) => {
                                        let broadcast_result = broadcast_request.execute(&*sdk_clone, RequestSettings::default()).await;
                                        Ok((transition_clone, broadcast_result))
                                    },
                                    Err(e) => Err(e)
                                }
                            };
                
                            broadcast_futures.push(future);
                        }
                
                        // Concurrently execute all broadcast requests
                        let broadcast_results = join_all(broadcast_futures).await;
                
                        // Create futures for waiting for state transition results
                        let mut wait_futures = Vec::new();
                        for (index, result) in broadcast_results.into_iter().enumerate() {
                            match result {
                                Ok((transition, broadcast_result)) => {
                                    if broadcast_result.is_err() {
                                        error!("Error broadcasting state transition {} for block height {}: {:?}", index + 1, current_block_height, broadcast_result.err().unwrap());
                                        continue;
                                    }
                
                                    let sdk_clone = Arc::clone(&sdk);
                                    let wait_future = async move {
                                        let wait_result = match transition.wait_for_state_transition_result_request() {
                                            Ok(wait_request) => wait_request.execute(&*sdk_clone, RequestSettings::default()).await,
                                            Err(e) => {
                                                error!("Error creating wait request for state transition {} block height {}: {:?}", index + 1, current_block_height, e);
                                                return None;
                                            }
                                        };
                
                                        match wait_result {
                                            Ok(wait_response) => {
                                                // Extract actual block height from the wait response
                                                if let Some(wait_for_state_transition_result_response::Version::V0(v0_response)) = wait_response.version {
                                                    v0_response.metadata.map(|metadata| metadata.height)
                                                } else {
                                                    None
                                                }
                                            }
                                            Err(_) => {
                                                None
                                            }
                                        }
                                    };
                                    wait_futures.push(wait_future);
                                },
                                Err(e) => {
                                    error!("Error preparing broadcast request for state transition {} block height {}: {:?}", index + 1, current_block_height, e);
                                }
                            }
                        }
                
                        // Wait for all state transition result futures to complete
                        let wait_results = join_all(wait_futures).await;
                
                        // Log the actual block height for each state transition
                        for (index, actual_block_height) in wait_results.into_iter().enumerate() {
                            match actual_block_height {
                                Some(height) => info!("Successfully processed state transition {} ({}) for block {} (actual block height: {})", index + 1, transition_type, current_block_height, height),
                                None => continue,
                            }
                        }
                    } else {
                        info!("No state transitions to process for block {}", current_block_height);

                        // // Wait until the blockchain height aligns with the next block height for state transitions
                        // loop {
                        //     match fetch_current_blockchain_height(&sdk).await {
                        //         Ok(blockchain_height) => {
                        //             if blockchain_height >= current_block_height {
                        //                 break;
                        //             }
                        //             info!("Waiting for blockchain to reach height {}. Current height: {}", current_block_height, blockchain_height);
                        //             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        //         },
                        //         Err(e) => {
                        //             error!("Error fetching current blockchain height: {}", e);
                        //             tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        //         }
                        //     }
                        // }
                    }
                                                
                    // Increment block height after processing each block
                    current_block_height += 1;
                }
            
                info!("Strategy '{}' finished running", strategy_name);
                                                                            
                BackendEvent::StrategyCompleted {
                    strategy_name: strategy_name.clone(),
                    result: StrategyCompletionResult::Success {
                        final_block_height: current_block_height,
                    }
                }
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RemoveLastContract(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;

            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                // Remove the last contract_with_update entry from the strategy
                strategy.contracts_with_updates.pop();

                // Also remove the corresponding entry from the displayed contracts
                if let Some(contract_names) = contract_names_lock.get_mut(&strategy_name) {
                    // Assuming each entry in contract_names corresponds to an entry in
                    // contracts_with_updates
                    contract_names.pop();
                }

                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(contract_names_lock, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RemoveIdentityInserts(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                strategy.identities_inserts = Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                };
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RemoveStartIdentities(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                strategy.start_identities = vec![];
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RemoveLastOperation(strategy_name) => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                strategy.operations.pop();
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(app_state.available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
    }
}

async fn set_start_identities(
    count: u16,
    key_count: u32,
    signer: &mut SimpleSigner,
    app_state: &AppState,
    sdk: &Sdk,
) -> Result<Vec<(Identity, StateTransition)>, Error> {
    let loaded_wallet = app_state.loaded_wallet.lock().await;
    let wallet_clone = loaded_wallet
        .clone()
        .ok_or_else(|| Error::WalletError(super::wallet::WalletError::Insight(InsightError("No wallet loaded".to_string()))))?;

    let wallet_ref = Arc::new(Mutex::new(wallet_clone)); // Use Arc<Mutex<Wallet>>

    // Define the create_asset_lock closure
    let mut create_asset_lock = move |amount: u64| -> Option<(AssetLockProof, PrivateKey)> {
        let wallet_clone = wallet_ref.clone(); // Clone the Arc<Mutex<Wallet>>, not the wallet itself

        // Use tokio::runtime::Runtime for executing async code
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async move {
            let mut wallet = wallet_clone.lock().await; // Lock the mutex to get mutable access
            let (asset_lock_transaction, asset_lock_proof_private_key) = match 
                wallet.asset_lock_transaction(None, amount) {
                    Ok(result) => result,
                    Err(_) => return None,
            };

            match AppState::broadcast_and_retrieve_asset_lock(sdk, &asset_lock_transaction, &wallet.receive_address()).await {
                Ok(proof) => Some((proof, asset_lock_proof_private_key)),
                Err(_) => None,
            }
        })
    };

    let identities_and_transitions = create_identities_state_transitions(
        count,
        key_count,
        signer,
        &mut StdRng::seed_from_u64(567),
        &mut create_asset_lock,
        PlatformVersion::latest(),
    )?;

    Ok(identities_and_transitions)
}

// Function to fetch current blockchain height
async fn fetch_current_blockchain_height(sdk: &Sdk) -> Result<u64, String> {
    let request = GetEpochsInfoRequest {
        version: Some(get_epochs_info_request::Version::V0(
            get_epochs_info_request::GetEpochsInfoRequestV0 {
                start_epoch: None,
                count: 1,
                ascending: false,
                prove: false,
            },
        )),
    };

    match sdk.execute(request, RequestSettings::default()).await {
        Ok(response) => {
            if let Some(get_epochs_info_response::Version::V0(response_v0)) = response.version {
                if let Some(metadata) = response_v0.metadata {
                    Ok(metadata.height)
                } else {
                    Err("Failed to get blockchain height: No metadata available".to_string())
                }
            } else {
                Err("Failed to get blockchain height: Incorrect response format".to_string())
            }
        },
        Err(e) => Err(format!("Failed to get blockchain height: {:?}", e)),
    }
}
