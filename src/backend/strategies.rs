//! Strategies management backend module.

use std::{collections::{BTreeMap, BTreeSet, HashMap, VecDeque}, sync::Arc};

use dapi_grpc::platform::v0::{GetEpochsInfoRequest, get_epochs_info_request, get_epochs_info_response};
use dash_platform_sdk::{Sdk, platform::transition::broadcast_request::BroadcastRequestForStateTransition};
use dpp::{
    data_contract::created_data_contract::CreatedDataContract, platform_value::{Bytes32, Identifier},
    version::PlatformVersion, block::{block_info::BlockInfo, epoch::Epoch}, identity::{Identity, PartialIdentity},
};
use drive::{drive::{identity::key::fetch::IdentityKeysRequest, document::query::{QueryDocumentsOutcome, QueryDocumentsOutcomeV0Methods}}, query::DriveQuery};
use futures::future::join_all;
use rand::{rngs::StdRng, SeedableRng};
use rs_dapi_client::{Dapi, RequestSettings, DapiRequest};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{
    frequency::Frequency, operations::Operation, transitions::create_identities_state_transitions,
    Strategy, LocalDocumentQuery, StrategyConfig,
};
use tokio::sync::MutexGuard;
use tracing::{info, error};

use super::{AppStateUpdate, BackendEvent, Task, AppState, StrategyCompletionResult};

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
            let mut contract_names_lock = app_state.available_strategies_contract_names.lock().await;

            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                let mut rng = StdRng::from_entropy();
                let platform_version = PlatformVersion::latest();

                if let Some(first_contract_name) = selected_contract_names.first() {
                    if let Some(data_contract) = known_contracts_lock.get(first_contract_name) {
                        let entropy = Bytes32::random_with_rng(&mut rng);
                        match CreatedDataContract::from_contract_and_entropy(
                            data_contract.clone(),
                            entropy,
                            platform_version,
                        ) {
                            Ok(initial_contract) => {
                                // Create a map for updates
                                let mut updates = BTreeMap::new();

                                // Process the subsequent contracts as updates
                                for (order, contract_name) in
                                    selected_contract_names.iter().enumerate().skip(1)
                                {
                                    if let Some(update_contract) =
                                        known_contracts_lock.get(contract_name)
                                    {
                                        let update_entropy = Bytes32::random_with_rng(&mut rng);
                                        match CreatedDataContract::from_contract_and_entropy(
                                            update_contract.clone(),
                                            update_entropy,
                                            platform_version,
                                        ) {
                                            Ok(created_update_contract) => {
                                                updates
                                                    .insert(order as u64, created_update_contract);
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "Error converting DataContract to \
                                                     CreatedDataContract for update: {:?}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }

                                // Add the initial contract and its updates as a new entry
                                strategy.contracts_with_updates.push((
                                    initial_contract,
                                    if updates.is_empty() {
                                        None
                                    } else {
                                        Some(updates)
                                    },
                                ));
                            }
                            Err(e) => {
                                eprintln!(
                                    "Error converting DataContract to CreatedDataContract: {:?}",
                                    e
                                );
                            }
                        }
                    }
                }

                // Transform the selected_contract_names into the expected format for display
                let mut transformed_contract_names = Vec::new();
                if let Some(first_contract_name) = selected_contract_names.first() {
                    let updates: BTreeMap<u64, String> = selected_contract_names
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(order, name)| (order as u64, name.clone()))
                        .collect();
                    transformed_contract_names.push((first_contract_name.clone(), Some(updates)));
                }

                // Check if there is an existing entry for the strategy
                if let Some(existing_contracts) = contract_names_lock.get_mut(&strategy_name) {
                    // Append the new transformed contracts to the existing list
                    existing_contracts.extend(transformed_contract_names);
                } else {
                    // If there is no existing entry, create a new one
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
            if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                tokio::task::block_in_place(|| set_start_identities(strategy, count, key_count));
                BackendEvent::TaskCompletedStateChange {
                    task: Task::Strategy(task.clone()),
                    execution_result: Ok("Start identities set".into()),
                    app_state_update: AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            app_state.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(strategy_name).expect("inconsistent data"),
                        ),
                    ),
                }
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::RunStrategy(strategy_name) => {
            info!("Starting strategy '{}'", strategy_name);

            let mut strategies_lock = app_state.available_strategies.lock().await;
            let drive_lock = app_state.drive.lock().await;
            let known_identities_lock = app_state.known_identities.lock().await;
            let loaded_identity_lock = app_state.loaded_identity.lock().await;
            let identity_private_keys_lock = app_state.identity_private_keys.lock().await;

            // Check if a loaded identity is available
            let loaded_identity = match &*loaded_identity_lock {
                Some(identity) => identity,
                None => {
                    return BackendEvent::StrategyError {
                        strategy_name: strategy_name.clone(),
                        error: "No loaded identity available for strategy execution".to_string(),
                    };
                }
            };

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
                            info!("Fetched initial block info successfully. Height {}", initial_block_info.height);
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

                // Get signer from loaded_identity
                // Convert loaded_identity to SimpleSigner
                let mut signer = SimpleSigner::default();
                match &*loaded_identity {
                    Identity::V0(identity_v0) => {
                        // Iterate over the public keys in the loaded identity
                        for (key_id, public_key) in &identity_v0.public_keys {
                            // Create a tuple of the identity ID and the key ID to match with the private keys
                            let identity_key_tuple = (identity_v0.id, *key_id);
                            if let Some(private_key_bytes) = identity_private_keys_lock.get(&identity_key_tuple) {
                                // Add the public key and its corresponding private key to the SimpleSigner
                                signer.private_keys.insert(public_key.clone(), private_key_bytes.to_bytes());
                            }
                        }
                    }
                }
                info!("Successfully created signer from loaded identity");
                                                                                
                // Get rng
                let mut rng = StdRng::from_entropy();

                // Create map of state transitions to block heights
                let mut state_transitions_map = HashMap::new();

                // Copy initial block info
                let mut block_info = initial_block_info.clone();

                // Fill out the state transitions map
                // Loop through each block to precompute state transitions
                for block_index in initial_block_info.height..(initial_block_info.height + num_blocks) {
                    info!("Precomputing state transitions for block {}", block_index);

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
                                                info!("Block {}: Fetched {} documents using DriveQuery", block_index, documents.len());
                                                documents
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        error!("Block {}: Error fetching documents using DriveQuery: {:?}", block_index, e);
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
                
                    // Get current identities
                    let mut current_identities: Vec<Identity> = known_identities_lock.values().cloned().collect();

                    // Call the function to get STs for block
                    let state_transitions = strategy.state_transitions_for_block_with_new_identities(
                        &mut document_query_callback, 
                        &mut identity_fetch_callback,
                        &block_info, 
                        &mut current_identities, 
                        &mut signer, 
                        &mut rng, 
                        PlatformVersion::latest(),
                        &StrategyConfig { start_block_height: initial_block_info.height },
                    );

                    // Store the state transitions in the map if not empty
                    if !state_transitions.0.is_empty() {
                        let st_queue = VecDeque::from(state_transitions.0);
                        state_transitions_map.insert(block_index, st_queue.clone());
                        info!("Prepared {} state transitions for block {}", st_queue.len(), block_index);
                    } else {
                        // Log when no state transitions are found for a block
                        info!("No state transitions prepared for block {}", block_index);
                    }

                    // Update block_info
                    block_info.height += 1;
                    block_info.time_ms += 1 * 1000;
                }
            
                let mut current_block_height = initial_block_info.height;

                info!("Starting strategy execution loop");
            
                // Create a vector to store futures for each state transition task
                let mut state_transition_futures = Vec::new();

                // Iterate over each block height
                for block_height in initial_block_info.height..(initial_block_info.height + num_blocks) {
                    info!("Processing block height {}", block_height);
                    current_block_height += 1;

                    if let Some(transitions) = state_transitions_map.get(&block_height) {
                        for transition in transitions {
                            let sdk_clone = Arc::clone(&sdk); // Efficiently clone the Arc
                            let transition_clone = transition.clone(); // Clone the transition if necessary

                            // Spawn a new task for broadcasting and waiting for each state transition
                            let transition_future = tokio::spawn(async move {
                                // Implement the logic for broadcasting the state transition
                                let broadcast_request = match transition_clone.broadcast_request_for_state_transition() {
                                    Ok(request) => request,
                                    Err(e) => {
                                        error!("Error creating broadcast request for block {}: {:?}", block_height, e);
                                        return Err(format!("Error creating broadcast request: {:?}", e));
                                    }
                                };
                                match broadcast_request.execute(&*sdk_clone, RequestSettings::default()).await {
                                    Ok(_) => {
                                        info!("Successfully broadcasted state transition for block {}", block_height);
                                        // Implement the logic for waiting for the transition result
                                        let wait_request = match transition_clone.wait_for_state_transition_result_request() {
                                            Ok(request) => request,
                                            Err(e) => {
                                                error!("Error creating wait request for block {}: {:?}", block_height, e);
                                                return Err(format!("Error creating wait request: {:?}", e));
                                            }
                                        };
                                        match wait_request.execute(&*sdk_clone, RequestSettings::default()).await {
                                            Ok(response) => {
                                                info!("Successfully received response for state transition for block {}", block_height);
                                                Ok(response)
                                            },
                                            Err(e) => {
                                                error!("Error executing wait request for block {}: {:?}", block_height, e);
                                                Err(format!("Error executing wait request: {:?}", e))
                                            },
                                        }
                                    }
                                    Err(e) => {
                                        error!("Error broadcasting state transition for block {}: {:?}", block_height, e);
                                        Err(format!("Error broadcasting state transition: {:?}", e))
                                    },
                                }
                            });

                            // Store the future in the vector
                            state_transition_futures.push(transition_future);
                        }
                    } else {
                        info!("No state transitions to process for block {}", block_height);
                    }

                    info!("Finished processing block height {}", block_height);
                }

                // Await all futures to complete
                let results = join_all(state_transition_futures).await;
                
                info!("Strategy '{}' finished running. Final block height: {}", strategy_name, current_block_height);

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

fn set_start_identities(strategy: &mut Strategy, count: u16, key_count: u32) {
    let identities = create_identities_state_transitions(
        count,
        key_count,
        &mut SimpleSigner::default(),
        &mut StdRng::seed_from_u64(567),
        PlatformVersion::latest(),
    );

    strategy.start_identities = identities;
}