//! Strategies management backend module.

use std::{collections::{BTreeMap, BTreeSet, HashMap, VecDeque}, sync::Arc, time::Instant};

use dapi_grpc::platform::v0::{GetEpochsInfoRequest, get_epochs_info_request, get_epochs_info_response, wait_for_state_transition_result_response::{self, wait_for_state_transition_result_response_v0}};
use rs_sdk::{Sdk, platform::transition::broadcast_request::BroadcastRequestForStateTransition};
use dpp::{
    data_contract::{created_data_contract::CreatedDataContract, DataContract, accessors::v0::DataContractV0Getters}, platform_value::{Bytes32, Identifier, string_encoding::Encoding},
    version::PlatformVersion, block::{block_info::BlockInfo, epoch::Epoch}, identity::{Identity, PartialIdentity, state_transition::asset_lock_proof::AssetLockProof, accessors::IdentityGettersV0}, state_transition::{StateTransition, data_contract_create_transition::DataContractCreateTransition}, dashcore::PrivateKey,
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

use crate::{backend::{wallet::WalletError, Backend, Wallet}, ui::Ui};

use super::{AppStateUpdate, BackendEvent, AppState, StrategyCompletionResult, error::Error, insight::{InsightError, InsightAPIClient}, Task};

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
    SetStartIdentities {
        strategy_name: String,
        count: u16,
        key_count: u32,
    },
    AddOperation {
        strategy_name: String,
        operation: Operation,
    },
    RunStrategy(String, u64),
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
            let mut selected_strategy_lock = app_state.selected_strategy.lock().await;

            strategies_lock.insert(
                strategy_name.clone(),
                Strategy::default(),
            );
            *selected_strategy_lock = Some(strategy_name.clone());
            contract_names_lock.insert(strategy_name.clone(), Default::default());

            BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                strategy_name.clone(),
                MutexGuard::map(strategies_lock, |strategies| {
                    strategies.get_mut(&strategy_name).expect("strategy exists")
                }),
                MutexGuard::map(contract_names_lock, |names| {
                    names.get_mut(&strategy_name).expect("inconsistent data")
                }),
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
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;
            let mut selected_strategy_lock = app_state.selected_strategy.lock().await;
        
            if let Some(selected_strategy_name) = &*selected_strategy_lock {
                if let Some(strategy_to_clone) = strategies_lock.get(selected_strategy_name) {
                    let cloned_strategy = strategy_to_clone.clone();        
                    let cloned_display_data = contract_names_lock
                        .get(selected_strategy_name)
                        .cloned()
                        .unwrap_or_default();
        
                    strategies_lock.insert(new_strategy_name.clone(), cloned_strategy);
                    contract_names_lock.insert(new_strategy_name.clone(), cloned_display_data);
                
                    *selected_strategy_lock = Some(new_strategy_name.clone());
                            
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        new_strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&new_strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(contract_names_lock, |names| {
                            names.get_mut(&new_strategy_name).expect("inconsistent data")
                        }),
                    ))
                } else {
                    BackendEvent::None
                }
            } else {
                BackendEvent::None
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
        StrategyTask::SetStartIdentities {
            ref strategy_name,
            count,
            key_count,
        } => {
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let loaded_identity_lock = app_state.loaded_identity.lock().await;
            let identity_private_keys_lock = app_state.identity_private_keys.lock().await;
        
            if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                // Ensure a signer is present, creating a new one if necessary
                let signer = if let Some(signer) = strategy.signer.as_mut() {
                    // Use the existing signer
                    signer
                } else {
                    // Create a new signer from loaded_identity if one doesn't exist, else default
                    let new_signer = if let Some(loaded_identity) = &*loaded_identity_lock {
                        let mut signer = SimpleSigner::default();
                        match loaded_identity {
                            Identity::V0(identity_v0) => {
                                for (key_id, public_key) in &identity_v0.public_keys {
                                    let identity_key_tuple = (identity_v0.id, *key_id);
                                    if let Some(private_key_bytes) = identity_private_keys_lock.get(&identity_key_tuple) {
                                        signer.private_keys.insert(public_key.clone(), private_key_bytes.to_bytes());
                                    }
                                }
                            }
                        }
                        signer
                    } else {
                        SimpleSigner::default()
                    };
                    strategy.signer = Some(new_signer);
                    strategy.signer.as_mut().unwrap()
                };
                                
                match set_start_identities(count, key_count, signer, app_state, &sdk, insight).await {
                    Ok(identities_and_transitions) => {
                        strategy.start_identities = identities_and_transitions;
                        BackendEvent::TaskCompletedStateChange {
                            task: Task::Strategy(task.clone()),
                            execution_result: Ok("Start identities set".into()),
                            app_state_update: AppStateUpdate::SelectedStrategy(
                                strategy_name.to_string(),
                                MutexGuard::map(strategies_lock, |strategies| {
                                    strategies.get_mut(strategy_name).expect("strategy exists")
                                }),
                                MutexGuard::map(
                                    app_state.available_strategies_contract_names.lock().await,
                                    |names| names.get_mut(strategy_name).expect("inconsistent data"),
                                ),
                            ),
                        }
                    },
                    Err(e) => {
                        error!("Error setting start identities: {:?}", e);
                        BackendEvent::StrategyError {
                            strategy_name: strategy_name.clone(),
                            error: format!("Error setting start identities: {}", e),
                        }
                    }
                }
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RunStrategy(strategy_name, num_blocks) => {
            info!("Starting strategy '{}'", strategy_name);
            let run_start_time = Instant::now();
        
            let mut strategies_lock = app_state.available_strategies.lock().await;
            let drive_lock = app_state.drive.lock().await;
            let identity_private_keys_lock = app_state.identity_private_keys.lock().await;
            let mut loaded_identity_lock = match app_state.refresh_identity(&sdk).await {
                Ok(lock) => lock,
                Err(e) => {
                    error!("Failed to refresh identity: {:?}", e);
                    return BackendEvent::StrategyError {
                        strategy_name: strategy_name.clone(),
                        error: format!("Failed to refresh identity: {:?}", e),
                    };
                }
            };
        
            let sdk_ref = Arc::clone(&sdk);
        
            // Access the loaded_wallet within the Mutex
            let mut loaded_wallet_lock = app_state.loaded_wallet.lock().await;

            // Refresh UTXOs for the loaded wallet
            if let Some(ref mut wallet) = *loaded_wallet_lock {
                wallet.reload_utxos(insight).await;
            }

            let initial_balance_identity = loaded_identity_lock.balance();
            let initial_balance_wallet = loaded_wallet_lock.clone().unwrap().balance();

            drop(loaded_wallet_lock);
                        
            // It's normal that we're asking for the mutable strategy because we need to modify
            // some properties of a contract on update
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {            
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
                // Use retry mechanism to fetch current block info
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
                                        
                // Create map of state transitions to block heights
                let mut state_transitions_map = HashMap::new();

                // Set initial current_identities to loaded_identity
                let mut current_identities: Vec<Identity> = vec![loaded_identity_lock.clone()];

                // Fill out the state transitions map
                // Loop through each block to precompute state transitions
                let mut current_block_info = initial_block_info.clone();
                let mut loaded_identity_clone = loaded_identity_lock.clone();
            
                // Loop through each block to precompute state transitions
                let prep_start_time = Instant::now();
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

                    let mut create_asset_lock = {
                        let loaded_wallet_clone = Arc::clone(&app_state.loaded_wallet);
                        let insight_ref = insight.clone();
                        let sdk_ref_clone = Arc::clone(&sdk_ref);
                    
                        move |amount: u64| -> Option<(AssetLockProof, PrivateKey)> {
                            // Use the current Tokio runtime to execute the async block
                            tokio::task::block_in_place(|| {
                                let rt = tokio::runtime::Handle::current();
                                rt.block_on(async {
                                    let mut wallet_lock = loaded_wallet_clone.lock().await;
                                    if let Some(ref mut wallet) = *wallet_lock {
                                        // Initialize old_utxos
                                        let old_utxos = match wallet {
                                            Wallet::SingleKeyWallet(ref wallet) => wallet.utxos.clone(),
                                        };

                                        // Handle asset lock transaction
                                        match wallet.asset_lock_transaction(None, amount) {
                                            Ok((asset_lock_transaction, asset_lock_proof_private_key)) => {
                                                // Use sdk_ref_clone for broadcasting and retrieving asset lock
                                                match AppState::broadcast_and_retrieve_asset_lock(&sdk_ref_clone, &asset_lock_transaction, &wallet.receive_address()).await {
                                                    Ok(proof) => {
                                                        let max_retries = 5;
                                                        let mut retries = 0;
                                                                        
                                                        let mut found_new_utxos = false;
                                                        while retries < max_retries {
                                                            wallet.reload_utxos(&insight_ref).await;
                                    
                                                            // Check if new UTXOs are available and if UTXO list is not empty
                                                            let current_utxos = match wallet {
                                                                Wallet::SingleKeyWallet(ref wallet) => &wallet.utxos,
                                                            };
                                                            if current_utxos != &old_utxos && !current_utxos.is_empty() {
                                                                found_new_utxos = true;
                                                                break;
                                                            } else {
                                                                retries += 1;
                                                                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                                                            }
                                                        }
                                    
                                                        if !found_new_utxos {
                                                            error!("Failed to find new UTXOs after maximum retries");
                                                            return None;
                                                        }
                                                                        
                                                        Some((proof, asset_lock_proof_private_key))
                                                    },
                                                    Err(e) => {
                                                        error!("Error broadcasting asset lock transaction: {:?}", e);
                                                        None
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                error!("Error creating asset lock transaction: {:?}", e);
                                                None
                                            }
                                        }
                                    } else {
                                        error!("Wallet not loaded");
                                        None
                                    }
                                })
                            })
                        }
                    };
                                                                                                                    
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
                            &StrategyConfig { 
                                start_block_height: initial_block_info.height,
                                number_of_blocks: num_blocks,
                            },
                            PlatformVersion::latest(),
                        )
                        .await;

                    // TO-DO: add documents from state transitions to explorer.drive here
                    // this is required for DocumentDelete and DocumentReplace strategy operations
                                                
                    // Process each FinalizeBlockOperation, which so far is just adding keys to the identities
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
                    if let Some(modified_identity) = current_identities.iter().find(|identity| identity.id() == loaded_identity_clone.id()) {
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
                                                                                                                                                                                            
                    // Update block_info
                    current_block_info.height += 1;
                    current_block_info.time_ms += 1 * 1000; // plus 1 second
                }

                let prep_time = prep_start_time.elapsed();
            
                let mut current_block_height = initial_block_info.height;

                info!("--- Starting strategy execution loop ---");

                let mut transition_count = 0;
                let mut success_count = 0;

                // Iterate over each block height
                while current_block_height < (initial_block_info.height + num_blocks) {                
                    if let Some(transitions) = state_transitions_map.get(&current_block_height) {
                        let mut broadcast_futures = Vec::new();
                        let mut transition_type = String::new();

                        for transition in transitions {
                            transition_count += 1;

                            let sdk_clone = Arc::clone(&sdk);
                            let transition_clone = transition.clone();

                            // Get ST type as string for logs
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
                                    Err(e) => {
                                        error!("Error creating broadcast request for state transition: {:?}", e);
                                        Err(e)
                                    }
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
                                                if let Some(wait_for_state_transition_result_response::Version::V0(v0_response)) = &wait_response.version {
                                                    if let Some(wait_for_state_transition_result_response_v0::Result::Error(error)) = &v0_response.result {
                                                        // Log the detailed state transition broadcast error
                                                        error!("State Transition Broadcast Error: Code: {}, Message: \"{}\"", error.code, error.message);
                                                    }
                                                }                                        

                                                // If a data contract was registered, add it to known_contracts
                                                if let StateTransition::DataContractCreate(DataContractCreateTransition::V0(data_contract_create_transition)) = &transition {
                                                    // Extract the data contract from the transition
                                                    let data_contract_serialized = &data_contract_create_transition.data_contract;
                                                    let data_contract_result = DataContract::try_from_platform_versioned(data_contract_serialized.clone(), false, PlatformVersion::latest());
                                                    
                                                    match data_contract_result {
                                                        Ok(data_contract) => {
                                                            // Get a mutable reference to app_state
                                                            let mut app_state_lock = app_state.known_contracts.lock().await;
                                                            app_state_lock.insert(data_contract.id().to_string(Encoding::Base58), data_contract);
                                                        },
                                                        Err(e) => {
                                                            error!("Error deserializing data contract: {:?}", e);
                                                        }
                                                    }
                                                }

                                                // Extract actual block height from the wait response
                                                if let Some(wait_for_state_transition_result_response::Version::V0(v0_response)) = wait_response.version {
                                                    v0_response.metadata.map(|metadata| metadata.height)
                                                } else {
                                                    error!("Error getting actual block height");
                                                    None
                                                }
                                            }
                                            Err(e) => {
                                                error!("Wait result error: {:?}", e);
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
                                Some(height) => {
                                    success_count += 1;
                                    info!("Successfully processed state transition {} ({}) for block {} (actual block height: {})", index + 1, transition_type, current_block_height, height)
                                },
                                None => {
                                    continue
                                },
                            }
                        }
                    } else {
                        info!("No state transitions to process for block {}", current_block_height);
                    }
                                                
                    // Increment block height after processing each block
                    current_block_height += 1;
                }

                // Clear start_identities since they have now been registered, if any
                if !strategy.start_identities.is_empty() {
                    strategy.start_identities = Vec::new();
                    info!("Strategy start_identities field cleared");
                }
            
                info!("Strategy '{}' finished running", strategy_name);
                let run_time = run_start_time.elapsed();

                // Refresh the identity at the end
                drop(loaded_identity_lock);
                let refresh_result = app_state.refresh_identity(&sdk).await;
                if let Err(ref e) = refresh_result {
                    error!("Failed to refresh identity after running strategy: {:?}", e);
                }

                // Attempt to retrieve the final balance from the refreshed identity
                let final_balance_identity = match refresh_result {
                    Ok(refreshed_identity_lock) => {
                        // Successfully refreshed, now access the balance
                        refreshed_identity_lock.balance()
                    },
                    Err(_) => {
                        error!("Error refreshing identity after running strategy");
                        initial_balance_identity
                    }
                };

                let dash_spent_identity = (initial_balance_identity as f64 - final_balance_identity as f64) / 100_000_000_000.0;

                let loaded_wallet_lock = app_state.loaded_wallet.lock().await;
                let final_balance_wallet = loaded_wallet_lock.clone().unwrap().balance();
                let dash_spent_wallet = (initial_balance_wallet as f64 - final_balance_wallet as f64) / 100_000_000_000.0;

                BackendEvent::StrategyCompleted {
                    strategy_name: strategy_name.clone(),
                    result: StrategyCompletionResult::Success {
                        final_block_height: current_block_height,
                        start_block_height: initial_block_info.height,
                        success_count,
                        transition_count,
                        prep_time,
                        run_time,
                        dash_spent_identity,
                        dash_spent_wallet,
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
    insight: &InsightAPIClient,
) -> Result<Vec<(Identity, StateTransition)>, Error> {
    // Lock the loaded wallet.
    let mut loaded_wallet = app_state.loaded_wallet.lock().await;

    // Refresh UTXOs for the loaded wallet
    if let Some(ref mut wallet) = *loaded_wallet {
        wallet.reload_utxos(insight).await;
    }


    // Clone wallet state
    let wallet_clone = loaded_wallet
        .clone()
        .ok_or_else(|| Error::WalletError(super::wallet::WalletError::Insight(InsightError("No wallet loaded".to_string()))))?;

    // Create a reference-counted, thread-safe clone of the wallet for the asset lock callback.
    let wallet_ref = Arc::new(Mutex::new(wallet_clone));

    // Define the asset lock callback.
    let mut create_asset_lock = move |amount: u64| -> Option<(AssetLockProof, PrivateKey)> {
        let wallet_clone = wallet_ref.clone();
        
        // Use block_in_place to execute the async code synchronously.
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut wallet = wallet_clone.lock().await;
    
                // Initialize old_utxos
                let old_utxos = match &*wallet {
                    Wallet::SingleKeyWallet(wallet) => wallet.utxos.clone(),
                };
    
                match wallet.asset_lock_transaction(None, amount) {
                    Ok((asset_lock_transaction, asset_lock_proof_private_key)) => {
                        match AppState::broadcast_and_retrieve_asset_lock(sdk, &asset_lock_transaction, &wallet.receive_address()).await {
                            Ok(proof) => {
                                // Implement retry logic for reloading UTXOs
                                let max_retries = 5;
                                let mut retries = 0;
                                let mut found_new_utxos = false;
    
                                while retries < max_retries {
                                    wallet.reload_utxos(insight).await;
    
                                    let current_utxos = match &*wallet {
                                        Wallet::SingleKeyWallet(wallet) => &wallet.utxos,
                                    };
                                    if current_utxos != &old_utxos && !current_utxos.is_empty() {
                                        found_new_utxos = true;
                                        break;
                                    } else {
                                        retries += 1;
                                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                                    }
                                }
    
                                if !found_new_utxos {
                                    error!("Failed to find new UTXOs after maximum retries");
                                    None
                                } else {
                                    Some((proof, asset_lock_proof_private_key))
                                }
                            },
                            Err(e) => {
                                error!("Error during asset lock transaction: {:?}", e);
                                None
                            },
                        }
                    },
                    Err(e) => {
                        error!("Error initiating asset lock transaction: {:?}", e);
                        match e {
                            WalletError::Balance => {
                                error!("Insufficient balance for asset lock transaction.");
                            },
                            _ => error!("Other wallet error occurred."),
                        }
                        None
                    },
                }
            })
        })
    };
    
    // Create identities and state transitions.
    let identities_and_transitions_result = create_identities_state_transitions(
        count,
        key_count,
        signer,
        &mut StdRng::from_entropy(),
        &mut create_asset_lock,
        PlatformVersion::latest(),
    );

    drop(loaded_wallet);

    // Check and reload UTXOs after the asset lock transactions, if successful.
    if identities_and_transitions_result.is_ok() {
        let mut loaded_wallet = app_state.loaded_wallet.lock().await;
        if let Some(ref mut wallet) = *loaded_wallet {
            if let Err(e) = reload_wallet_utxos(wallet, insight).await {
                error!("Error reloading wallet UTXOs after asset lock: {:?}", e);
                return Err(e);
            }
        }
    }

    Ok(identities_and_transitions_result?)
}


async fn reload_wallet_utxos(wallet: &mut Wallet, insight: &InsightAPIClient) -> Result<(), Error> {    
    let old_utxos = match wallet {
        Wallet::SingleKeyWallet(ref single_key_wallet) => single_key_wallet.utxos.clone(),
    };

    let max_retries = 5;
    let mut retries = 0;

    while retries < max_retries {
        wallet.reload_utxos(insight).await;

        let new_utxos = match wallet {
            Wallet::SingleKeyWallet(ref single_key_wallet) => &single_key_wallet.utxos,
            // Handle other wallet types if necessary
        };

        if new_utxos != &old_utxos && !new_utxos.is_empty() {
            return Ok(());
        } else {
            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    Err(Error::SdkError(rs_sdk::Error::Generic("Failed to reload wallet UTXOs after maximum retries".to_string())))
}

// // Function to fetch current blockchain height
// async fn fetch_current_blockchain_height(sdk: &Sdk) -> Result<u64, String> {
//     let request = GetEpochsInfoRequest {
//         version: Some(get_epochs_info_request::Version::V0(
//             get_epochs_info_request::GetEpochsInfoRequestV0 {
//                 start_epoch: None,
//                 count: 1,
//                 ascending: false,
//                 prove: false,
//             },
//         )),
//     };

//     match sdk.execute(request, RequestSettings::default()).await {
//         Ok(response) => {
//             if let Some(get_epochs_info_response::Version::V0(response_v0)) = response.version {
//                 if let Some(metadata) = response_v0.metadata {
//                     Ok(metadata.height)
//                 } else {
//                     Err("Failed to get blockchain height: No metadata available".to_string())
//                 }
//             } else {
//                 Err("Failed to get blockchain height: Incorrect response format".to_string())
//             }
//         },
//         Err(e) => Err(format!("Failed to get blockchain height: {:?}", e)),
//     }
// }
