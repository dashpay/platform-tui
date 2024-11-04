//! Strategies management backend module.

use std::{
    cmp::min,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::File,
    io::Write,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossterm::style::Stylize;
use dapi_grpc::{
    platform::v0::{
        get_epochs_info_request, get_epochs_info_response,
        wait_for_state_transition_result_response::{
            self, wait_for_state_transition_result_response_v0,
        },
        GetEpochsInfoRequest,
    },
    tonic::Code,
};
use dash_sdk::dashcore_rpc::Client;
use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
use dash_sdk::{
    platform::{transition::broadcast_request::BroadcastRequestForStateTransition, Fetch},
    Sdk,
};
use dashmap::DashMap;
use dpp::fee::Credits;
use dpp::{
    block::{block_info::BlockInfo, epoch::Epoch},
    dashcore::{Address, PrivateKey, Transaction},
    data_contract::{
        accessors::v0::{DataContractV0Getters, DataContractV0Setters},
        created_data_contract::CreatedDataContract,
        document_type::random_document::{DocumentFieldFillSize, DocumentFieldFillType},
        DataContract,
    },
    data_contracts::{dashpay_contract, dpns_contract},
    document::Document,
    identity::{
        accessors::IdentityGettersV0, state_transition::asset_lock_proof::AssetLockProof, Identity,
        KeyType, PartialIdentity, Purpose, SecurityLevel,
    },
    platform_value::{string_encoding::Encoding, Identifier},
    serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructure,
        PlatformSerializableWithPlatformVersion,
    },
    state_transition::{
        data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0,
        documents_batch_transition::{
            document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods,
            document_create_transition::v0::DocumentFromCreateTransitionV0,
            document_transition::DocumentTransition, DocumentCreateTransition,
            DocumentDeleteTransition, DocumentsBatchTransition,
        },
        identity_topup_transition::{
            methods::IdentityTopUpTransitionMethodsV0, IdentityTopUpTransition,
        },
        StateTransition, StateTransitionLike,
    },
};
use drive::{
    drive::{
        document::query::{QueryDocumentsOutcome, QueryDocumentsOutcomeV0Methods},
        identity::key::fetch::IdentityKeysRequest,
        Drive,
    },
    error::proof::ProofError,
    query::DriveDocumentQuery,
    util::object_size_info::{DocumentInfo, OwnedDocumentInfo},
};
use futures::{future::join_all, stream::FuturesUnordered, FutureExt};
use hdrhistogram::Histogram;
use itertools::Itertools;
use rand::{rngs::StdRng, SeedableRng};
use rs_dapi_client::{DapiRequest, DapiRequestExecutor, RequestSettings};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{
    frequency::Frequency,
    operations::{DocumentAction, DocumentOp, FinalizeBlockOperation, Operation, OperationType},
    IdentityInsertInfo, LocalDocumentQuery, StartIdentities, Strategy, StrategyConfig,
};
use tokio::sync::{oneshot, Mutex, MutexGuard, Semaphore};

use crate::backend::{wallet::SingleKeyWallet, Wallet};

use super::{
    insight::InsightAPIClient,
    state::{ContractFileName, KnownContractsMap},
    AppState, AppStateUpdate, BackendEvent, StrategyCompletionResult, StrategyContractNames, Task,
};

#[derive(Debug, PartialEq, Clone)]
pub enum StrategyTask {
    CreateStrategy(String),
    ImportStrategy(String),
    ExportStrategy(String),
    SelectStrategy(String),
    DeleteStrategy(String),
    CloneStrategy(String),
    SetStartContracts(String, Vec<String>),
    SetStartContractsRandom(String, String, u8),
    SetIdentityInserts {
        strategy_name: String,
        identity_inserts_frequency: Frequency,
        start_balance: Credits,
    },
    SetStartIdentities {
        strategy_name: String,
        count: u16,
        keys_count: u8,
        balance: u64,
        add_transfer_key: bool,
    },
    AddHardCodedStartIdentity {
        strategy_name: String,
        identity_id_str: String,
    },
    SetStartIdentitiesBalance(String, u64),
    AddOperation {
        strategy_name: String,
        operation: Operation,
    },
    RegisterDocsToAllContracts(String, u16, DocumentFieldFillSize, DocumentFieldFillType),
    RunStrategy(String, u64, u64, bool, u64),
    RemoveLastContract(String),
    ClearContracts(String),
    ClearOperations(String),
    RemoveIdentityInserts(String),
    RemoveStartIdentities(String),
    RemoveLastOperation(String),
}

impl AppState {
    pub async fn run_strategy_task<'s>(
        &'s self,
        sdk: &Sdk,
        task: StrategyTask,
        insight: &'s InsightAPIClient,
        core_client: &'s Client,
    ) -> BackendEvent<'s> {
        match task {
            StrategyTask::CreateStrategy(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;
                let mut selected_strategy_lock = self.selected_strategy.lock().await;

                strategies_lock.insert(strategy_name.clone(), Strategy::default());
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
            StrategyTask::ImportStrategy(url) => {
                match reqwest::get(&url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => {
                                    match Strategy::versioned_deserialize(
                                        &bytes,
                                        true,
                                        &sdk.version(),
                                    ) {
                                        Ok(strategy) => {
                                            let strategy_name = url.split('/').last()
                                                .map(|s| s.rsplit_once('.').map_or(s, |(name, _)| name))
                                                .map(|s| s.to_string())
                                                .expect("Expected to extract the filename from the imported Strategy file");

                                            let mut strategies_lock =
                                                self.available_strategies.lock().await;
                                            strategies_lock
                                                .insert(strategy_name.clone(), strategy.clone());

                                            // We need to add the contracts to available_strategies_contract_names so they can be displayed.
                                            // In order to do so, we need to convert start_contracts into Base58-encoded IDs
                                            let mut strategy_start_contracts_in_format: StrategyContractNames = Vec::new();
                                            for (contract, maybe_updates) in
                                                strategy.start_contracts
                                            {
                                                let contract_name = contract
                                                    .data_contract()
                                                    .id()
                                                    .to_string(Encoding::Base58);
                                                if let Some(update_map) = maybe_updates {
                                                    let formatted_update_map = update_map
                                                        .into_iter()
                                                        .map(|(block_number, created_contract)| {
                                                            let contract_name = created_contract
                                                                .data_contract()
                                                                .id()
                                                                .to_string(Encoding::Base58);
                                                            (block_number, contract_name)
                                                        })
                                                        .collect::<BTreeMap<u64, ContractFileName>>(
                                                        );

                                                    strategy_start_contracts_in_format.push((
                                                        contract_name,
                                                        Some(formatted_update_map),
                                                    ));
                                                } else {
                                                    strategy_start_contracts_in_format
                                                        .push((contract_name, None));
                                                }
                                            }

                                            let mut contract_names_lock = self
                                                .available_strategies_contract_names
                                                .lock()
                                                .await;
                                            contract_names_lock.insert(
                                                strategy_name.clone(),
                                                strategy_start_contracts_in_format,
                                            );

                                            let mut selected_strategy =
                                                self.selected_strategy.lock().await;
                                            *selected_strategy = Some(strategy_name.clone());

                                            BackendEvent::AppStateUpdated(
                                                AppStateUpdate::SelectedStrategy(
                                                    strategy_name.clone(),
                                                    MutexGuard::map(
                                                        strategies_lock,
                                                        |strategies| {
                                                            strategies.get_mut(&strategy_name).expect("Expected to find the strategy in available_strategies")
                                                        },
                                                    ),
                                                    MutexGuard::map(contract_names_lock, |names| {
                                                        names.get_mut(&strategy_name).expect("Expected to find the strategy in available_strategies_contract_names")
                                                    }),
                                                ),
                                            )
                                        }
                                        Err(e) => {
                                            tracing::debug!(
                                                "Failed to deserialize strategy: {}",
                                                e
                                            );
                                            BackendEvent::StrategyError {
                                                error: format!(
                                                    "Failed to deserialize strategy: {}",
                                                    e
                                                ),
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!("Failed to fetch strategy data: {}", e);
                                    BackendEvent::StrategyError {
                                        error: format!("Failed to fetch strategy data: {}", e),
                                    }
                                }
                            }
                        } else {
                            tracing::debug!("Failed to fetch strategy: HTTP {}", response.status());
                            BackendEvent::StrategyError {
                                error: format!(
                                    "Failed to fetch strategy: HTTP {}",
                                    response.status()
                                ),
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Failed to fetch strategy: {}", e);
                        BackendEvent::StrategyError {
                            error: format!("Failed to fetch strategy: {}", e),
                        }
                    }
                }
            }
            StrategyTask::ExportStrategy(ref strategy_name) => {
                let strategies_lock = self.available_strategies.lock().await;
                let strategy = strategies_lock
                    .get(strategy_name)
                    .expect("Strategy name doesn't exist in self.available_strategies");
                let platform_version = sdk.version();

                match strategy.serialize_to_bytes_with_platform_version(&platform_version) {
                    Ok(binary_data) => {
                        let file_name =
                            format!("supporting_files/strategy_exports/{}", strategy_name);
                        let path = std::path::Path::new(&file_name);

                        match File::create(&path) {
                            Ok(mut file) => {
                                if let Err(e) = file.write_all(&binary_data) {
                                    tracing::debug!("Failed to write strategy to file: {}", e);
                                    return BackendEvent::StrategyError {
                                        error: format!("Failed to write strategy to file: {}", e),
                                    };
                                }
                                BackendEvent::TaskCompleted {
                                    task: Task::Strategy(task),
                                    execution_result: Ok(format!(
                                        "Exported strategy file to supporting_files/strategy_exports"
                                    )
                                    .into()),
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Failed to create file: {}", e);
                                BackendEvent::StrategyError {
                                    error: format!("Failed to create file: {}", e),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Failed to serialize strategy: {}", e);
                        BackendEvent::StrategyError {
                            error: format!("Failed to serialize strategy: {}", e),
                        }
                    }
                }
            }
            StrategyTask::SelectStrategy(ref strategy_name) => {
                let mut selected_strategy_lock = self.selected_strategy.lock().await;
                let strategies_lock = self.available_strategies.lock().await;

                if strategies_lock.contains_key(strategy_name) {
                    *selected_strategy_lock = Some(strategy_name.clone());
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::DeleteStrategy(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;
                let mut selected_strategy_lock = self.selected_strategy.lock().await;

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
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::CloneStrategy(new_strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;
                let mut selected_strategy_lock = self.selected_strategy.lock().await;

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
                                strategies
                                    .get_mut(&new_strategy_name)
                                    .expect("strategy exists")
                            }),
                            MutexGuard::map(contract_names_lock, |names| {
                                names
                                    .get_mut(&new_strategy_name)
                                    .expect("inconsistent data")
                            }),
                        ))
                    } else {
                        BackendEvent::StrategyError {
                            error: format!("Strategy doesn't exist in app state."),
                        }
                    }
                } else {
                    BackendEvent::StrategyError {
                        error: format!("No selected strategy in app state."),
                    }
                }
            }
            StrategyTask::SetStartContracts(strategy_name, selected_contract_names) => {
                // Attain state locks
                let mut strategies_lock = self.available_strategies.lock().await;
                let known_contracts_lock = self.known_contracts.lock().await;
                let supporting_contracts_lock = self.supporting_contracts.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;

                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    let platform_version = sdk.version();

                    // Function to retrieve the contract from either known_contracts or
                    // supporting_contracts
                    let get_contract = |contract_name: &String| {
                        known_contracts_lock
                            .get(contract_name)
                            .or_else(|| supporting_contracts_lock.get(contract_name))
                            .cloned()
                    };

                    // Set a fake identity nonce for now. We will set real identity nonces during strategy execution.
                    let fake_identity_nonce = 1;

                    if let Some(first_contract_name) = selected_contract_names.first() {
                        if let Some(mut data_contract) = get_contract(first_contract_name) {
                            data_contract.set_version(1);

                            match CreatedDataContract::from_contract_and_identity_nonce(
                                data_contract,
                                fake_identity_nonce,
                                platform_version,
                            ) {
                                Ok(original_contract) => {
                                    let mut updates = BTreeMap::new();

                                    for (order, contract_name) in
                                        selected_contract_names.iter().enumerate().skip(1)
                                    {
                                        if let Some(update_contract) = get_contract(contract_name) {
                                            match CreatedDataContract::from_contract_and_identity_nonce(
                                                update_contract,
                                                fake_identity_nonce,
                                                platform_version,
                                            ) {
                                                Ok(created_update_contract) => {
                                                    updates
                                                        .insert(order as u64, created_update_contract);
                                                }
                                                Err(e) => {
                                                    tracing::debug!(
                                                        "Error converting DataContract to \
                                                         CreatedDataContract for update: {:?}",
                                                        e
                                                    );
                                                    return BackendEvent::StrategyError {
                                                        error: format!("Error converting DataContract to CreatedDataContract for update: {:?}", e)
                                                    };
                                                }
                                            }
                                        }
                                    }

                                    strategy.start_contracts.push((
                                        original_contract,
                                        if updates.is_empty() {
                                            None
                                        } else {
                                            Some(updates)
                                        },
                                    ));
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Error converting DataContract to CreatedDataContract: {:?}",
                                        e
                                    );
                                    return BackendEvent::StrategyError {
                                        error: format!("Error converting DataContract to CreatedDataContract: {:?}", e)
                                    };
                                }
                            }
                        }
                    }

                    let mut transformed_contract_names = Vec::new();
                    if let Some(first_contract_name) = selected_contract_names.first() {
                        let updates: BTreeMap<u64, String> = selected_contract_names
                            .iter()
                            .enumerate()
                            .skip(1)
                            .map(|(order, name)| (order as u64, name.clone()))
                            .collect();
                        transformed_contract_names
                            .push((first_contract_name.clone(), Some(updates)));
                    }

                    if let Some(existing_contracts) = contract_names_lock.get_mut(&strategy_name) {
                        existing_contracts.extend(transformed_contract_names);
                    } else {
                        contract_names_lock
                            .insert(strategy_name.clone(), transformed_contract_names);
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
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::SetStartContractsRandom(
                strategy_name,
                selected_contract_name,
                variants,
            ) => {
                // Attain state locks
                let mut strategies_lock = self.available_strategies.lock().await;
                let known_contracts_lock = self.known_contracts.lock().await;
                let mut supporting_contracts_lock = self.supporting_contracts.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;

                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    let platform_version = sdk.version();

                    // Function to retrieve the contract from either known_contracts or
                    // supporting_contracts
                    let get_contract = |contract_name: &String| {
                        known_contracts_lock
                            .get(contract_name)
                            .or_else(|| supporting_contracts_lock.get(contract_name))
                            .cloned()
                    };

                    // Set a fake identity nonce for now. We will set real identity nonces during strategy execution.
                    let mut fake_identity_nonce = 1;

                    // Add the contracts to the strategy start_contracts
                    if let Some(mut data_contract) = get_contract(&selected_contract_name) {
                        data_contract.set_version(1);

                        match CreatedDataContract::from_contract_and_identity_nonce(
                            data_contract,
                            fake_identity_nonce,
                            platform_version,
                        ) {
                            Ok(original_contract) => {
                                // Add original contract to the strategy
                                let mut contract_variants: Vec<CreatedDataContract> = Vec::new();
                                contract_variants.push(original_contract.clone());
                                strategy
                                    .start_contracts
                                    .push((original_contract.clone(), None));

                                // Add i variants of the original contract to the strategy
                                for i in 0..variants - 1 {
                                    let mut new_data_contract =
                                        original_contract.data_contract().clone();
                                    let new_id = DataContract::generate_data_contract_id_v0(
                                        Identifier::random(),
                                        fake_identity_nonce,
                                    );
                                    new_data_contract.set_id(new_id);
                                    match CreatedDataContract::from_contract_and_identity_nonce(
                                        new_data_contract.clone(),
                                        fake_identity_nonce,
                                        platform_version,
                                    ) {
                                        Ok(contract) => {
                                            contract_variants.push(contract.clone());
                                            strategy.start_contracts.push((contract, None));
                                            let new_contract_name = String::from(format!(
                                                "{}_variant_{}",
                                                selected_contract_name, i
                                            ));
                                            // Insert into supporting_contracts so we can register documents to them. We will clear
                                            // supporting contracts at the end of strategy execution.
                                            supporting_contracts_lock
                                                .insert(new_contract_name, new_data_contract);
                                            fake_identity_nonce += 1;
                                        }
                                        Err(e) => {
                                            tracing::debug!(
                                                "Error converting DataContract to CreatedDataContract variant: {:?}",
                                                e
                                            );
                                            return BackendEvent::StrategyError {
                                                error: format!("Error converting DataContract to CreatedDataContract variant: {:?}", e)
                                            };
                                        }
                                    };
                                }

                                let contract_id_strings: Vec<(
                                    String,
                                    Option<BTreeMap<u64, String>>,
                                )> = contract_variants
                                    .iter()
                                    .map(|x| {
                                        (x.data_contract().id().to_string(Encoding::Base58), None)
                                    })
                                    .collect();

                                // Add the new contracts to self.available_strategies_contract_names
                                if let Some(existing_strategy_contracts) =
                                    contract_names_lock.get_mut(&strategy_name)
                                {
                                    existing_strategy_contracts.extend(contract_id_strings);
                                } else {
                                    contract_names_lock
                                        .insert(strategy_name.clone(), contract_id_strings);
                                }
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Error converting original DataContract to CreatedDataContract: {:?}",
                                    e
                                );
                                return BackendEvent::StrategyError {
                                    error: format!("Error converting original DataContract to CreatedDataContract: {:?}", e)
                                };
                            }
                        }
                    } else {
                        tracing::debug!("Contract wasn't retrieved by name in StrategyTask::SetContractsWithUpdatesRandom");
                        return BackendEvent::StrategyError {
                            error: format!("Contract wasn't retrieved by name in StrategyTask::SetContractsWithUpdatesRandom")
                        };
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
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::AddOperation {
                ref strategy_name,
                ref operation,
            } => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                    strategy.operations.push(operation.clone());
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::RegisterDocsToAllContracts(
                strategy_name,
                num_docs,
                fill_size,
                fill_type,
            ) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    for contract_with_updates in &strategy.start_contracts {
                        let contract = &contract_with_updates.0;
                        let document_types = contract.data_contract().document_types();
                        let document_type = document_types.values().next().expect(
                            "Expected to get a document type in RegisterDocsToAllContracts",
                        );
                        let action =
                            DocumentAction::DocumentActionInsertRandom(fill_type, fill_size);
                        let operation = Operation {
                            op_type: OperationType::Document(DocumentOp {
                                contract: contract.data_contract().clone(),
                                document_type: document_type.clone(),
                                action,
                            }),
                            frequency: Frequency {
                                times_per_block_range: num_docs..num_docs + 1,
                                chance_per_block: None,
                            },
                        };
                        strategy.operations.push(operation);
                    }
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::SetIdentityInserts {
                strategy_name,
                identity_inserts_frequency,
                start_balance,
            } => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    strategy.identity_inserts = IdentityInsertInfo {
                        frequency: identity_inserts_frequency,
                        start_keys: 3,
                        extra_keys: BTreeMap::new(),
                        start_balance_range: start_balance..=start_balance,
                    };
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::SetStartIdentities {
                strategy_name,
                count,
                keys_count,
                balance,
                add_transfer_key,
            } => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    let mut extra_keys = BTreeMap::new();
                    if add_transfer_key {
                        extra_keys.insert(
                            Purpose::TRANSFER,
                            [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                        );
                    }
                    strategy.start_identities = StartIdentities {
                        number_of_identities: count as u16,
                        keys_per_identity: keys_count,
                        starting_balances: balance,
                        extra_keys,
                        hard_coded: vec![],
                    };
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::AddHardCodedStartIdentity {
                strategy_name,
                identity_id_str,
            } => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    let known_identities = self.known_identities.lock().await;
                    let maybe_identity = known_identities
                        .get(
                            &Identifier::from_string(&identity_id_str, Encoding::Base58)
                                .expect("Expected to convert string to Identifier"),
                        )
                        .cloned();
                    if let Some(identity) = maybe_identity {
                        strategy.start_identities.hard_coded.push((identity, None));
                        BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                            strategy_name.clone(),
                            MutexGuard::map(strategies_lock, |strategies| {
                                strategies.get_mut(&strategy_name).expect("strategy exists")
                            }),
                            MutexGuard::map(
                                self.available_strategies_contract_names.lock().await,
                                |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                            ),
                        ))
                    } else {
                        BackendEvent::StrategyError {
                            error: format!("Identity with id {} not found", identity_id_str),
                        }
                    }
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::SetStartIdentitiesBalance(strategy_name, balance) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    strategy.start_identities = StartIdentities {
                        number_of_identities: strategy.start_identities.number_of_identities,
                        keys_per_identity: strategy.start_identities.keys_per_identity,
                        starting_balances: balance,
                        extra_keys: strategy.start_identities.extra_keys.clone(),
                        hard_coded: strategy.start_identities.hard_coded.clone(),
                    };
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state."),
                    }
                }
            }
            StrategyTask::RunStrategy(
                strategy_name,
                duration,
                seconds_per_loop,
                verify_proofs,
                top_up_amount,
            ) => {
                tracing::info!("-----Starting strategy '{}'-----", strategy_name);
                let init_start_time = Instant::now(); // Start time of strategy initialization plus execution of first two blocks
                let mut init_time = Duration::new(0, 0); // Will set this to the time it takes for all initialization plus the first two blocks to complete

                // Fetch known_contracts from the chain to assure local copies match actual state.
                match AppState::update_known_contracts(&self, sdk, &self.known_contracts).await {
                    Ok(_) => {
                        // nothing
                    }
                    Err(e) => {
                        tracing::debug!("Failed to update known contracts: {:?}", e);
                        return BackendEvent::StrategyError {
                            error: format!("Failed to update known contracts: {:?}", e),
                        };
                    }
                };

                // Refresh loaded_identity and get the current balance at strategy start
                let mut loaded_identity_lock = match self.refresh_loaded_identity(&sdk).await {
                    Ok(lock) => lock,
                    Err(e) => {
                        tracing::debug!("Failed to refresh loaded identity: {:?}", e);
                        return BackendEvent::StrategyError {
                            error: format!("Failed to refresh loaded identity: {:?}", e),
                        };
                    }
                };
                let initial_balance_identity = loaded_identity_lock.balance();

                // Refresh UTXOs for the loaded wallet and get initial wallet balance
                let mut loaded_wallet_lock = self.loaded_wallet.lock().await;
                if let Some(ref mut wallet) = *loaded_wallet_lock {
                    let _ = wallet.reload_utxos(insight, core_client).await;
                }
                let initial_balance_wallet = loaded_wallet_lock.clone().unwrap().balance();
                drop(loaded_wallet_lock);

                // Get a mutable strategy because we need to modify some properties of contracts on updates
                let mut strategies_lock = self.available_strategies.lock().await;
                if strategies_lock.get_mut(&strategy_name).is_none() {
                    return BackendEvent::StrategyError {
                        error: "No known strategy with that name in app state".to_string(),
                    };
                };
                let strategy = strategies_lock
                    .get_mut(&strategy_name)
                    .expect("Expected to get a strategy with that name");

                // Get block_info
                // Get block info for the first block by sending a grpc request and looking at
                // the metadata Retry up to MAX_RETRIES times
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
                    match sdk
                        .execute(request.clone(), RequestSettings::default())
                        .await
                    {
                        Ok(response) => {
                            if let Some(get_epochs_info_response::Version::V0(response_v0)) =
                                response.inner.version
                            {
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
                        }
                        Err(e) if retries < MAX_RETRIES => {
                            tracing::debug!("Error executing request, retrying: {:?}", e);
                            retries += 1;
                        }
                        Err(e) => {
                            tracing::debug!("Failed to execute request after retries: {:?}", e);
                            return BackendEvent::StrategyError {
                                error: format!("Failed to execute request after retries: {:?}", e),
                            };
                        }
                    }
                }
                initial_block_info.height += 1; // Add one because we'll be submitting to the next block

                // Get signer from loaded_identity
                // Convert loaded_identity to SimpleSigner
                let identity_private_keys_lock = self.known_identities_private_keys.lock().await;
                let mut signer = {
                    let strategy_signer = strategy.signer.insert({
                        let mut new_signer = SimpleSigner::default();
                        let Identity::V0(identity_v0) = &*loaded_identity_lock;
                        for (key_id, public_key) in &identity_v0.public_keys {
                            let identity_key_tuple = (identity_v0.id, *key_id);
                            if let Some(private_key_bytes) =
                                identity_private_keys_lock.get(&identity_key_tuple)
                            {
                                new_signer
                                    .private_keys
                                    .insert(public_key.clone(), private_key_bytes.clone());
                            }
                        }
                        new_signer
                    });
                    strategy_signer.clone()
                };
                drop(identity_private_keys_lock);

                // Set initial current_identities to loaded_identity
                // During strategy execution, newly created identities will be added to current_identities
                let mut loaded_identity_clone = loaded_identity_lock.clone();
                let current_identities = Arc::new(Mutex::new(vec![loaded_identity_clone.clone()]));

                // Add the hardcoded start identities that are already created to current_identities
                let hard_coded_start_identities = strategy
                    .start_identities
                    .hard_coded
                    .iter()
                    .filter_map(|(identity, transition)| {
                        if transition.is_none() {
                            Some(identity.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Identity>>();

                {
                    current_identities
                        .lock()
                        .await
                        .extend(hard_coded_start_identities);
                }

                // Add the loaded identity to hardcoded start identities if it's not already present
                if !strategy
                    .start_identities
                    .hard_coded
                    .iter()
                    .any(|(identity, _)| identity.id() == loaded_identity_lock.id())
                {
                    strategy
                        .start_identities
                        .hard_coded
                        .push((loaded_identity_lock.clone(), None));
                }

                // Set the nonce counters
                let used_contract_ids = strategy.used_contract_ids();
                let mut identity_nonce_counter = BTreeMap::new();
                tracing::info!(
                    "Fetching identity nonce and {} identity contract nonces from Platform...",
                    used_contract_ids.len()
                );
                let nonce_fetching_time = Instant::now();
                let identity_future = sdk.get_identity_nonce(
                    loaded_identity_clone.id(),
                    false,
                    Some(dash_sdk::platform::transition::put_settings::PutSettings {
                        request_settings: RequestSettings::default(),
                        identity_nonce_stale_time_s: Some(0),
                        user_fee_increase: None,
                    }),
                );
                let contract_futures =
                    used_contract_ids
                        .clone()
                        .into_iter()
                        .map(|used_contract_id| {
                            let identity_id = loaded_identity_clone.id();
                            async move {
                                let current_nonce = sdk.get_identity_contract_nonce(
                            identity_id,
                            used_contract_id,
                            false,
                            Some(dash_sdk::platform::transition::put_settings::PutSettings {
                                request_settings: RequestSettings::default(),
                                identity_nonce_stale_time_s: Some(0),
                                user_fee_increase: None,
                            })
                        ).await.expect("Couldn't get current identity contract nonce");
                                ((identity_id, used_contract_id), current_nonce)
                            }
                        });
                let identity_result = match identity_future.await {
                    Ok(nonce) => nonce,
                    Err(e) => {
                        return BackendEvent::StrategyError {
                            error: format!("Failed to fetch identity nonce: {:?}", e),
                        };
                    }
                };

                identity_nonce_counter.insert(loaded_identity_clone.id(), identity_result);
                let contract_results = join_all(contract_futures).await;
                let mut contract_nonce_counter: BTreeMap<(Identifier, Identifier), u64> =
                    contract_results.into_iter().collect();

                // Get a lock on the local drive for the following two callbacks
                let drive_lock = self.drive.lock().await;

                // Callback used to fetch documents from the local Drive instance
                // Used for DocumentReplace and DocumentDelete transitions
                let mut document_query_callback = |query: LocalDocumentQuery| {
                    match query {
                        LocalDocumentQuery::RandomDocumentQuery(random_query) => {
                            let document_type = random_query.document_type;
                            let data_contract = random_query.data_contract;

                            // Construct a DriveQuery based on the document_type and
                            // data_contract
                            let drive_query = DriveDocumentQuery::any_item_query(
                                data_contract,
                                document_type.as_ref(),
                            );

                            // Query the Drive for documents
                            match drive_lock.query_documents(drive_query, None, false, None, None) {
                                Ok(outcome) => match outcome {
                                    QueryDocumentsOutcome::V0(outcome_v0) => {
                                        let documents = outcome_v0.documents_owned();
                                        documents
                                    }
                                },
                                Err(e) => {
                                    tracing::debug!(
                                        "Error fetching documents using DriveQuery: {:?}",
                                        e
                                    );
                                    vec![]
                                }
                            }
                        }
                    }
                };

                // Callback used to fetch identities from the local Drive instance
                // Is this ever used?
                let mut identity_fetch_callback =
                    |identifier: Identifier, _keys_request: Option<IdentityKeysRequest>| {
                        // Convert Identifier to a byte array format expected by the Drive
                        // method
                        let identity_id_bytes = identifier.into_buffer();

                        // Fetch identity information from the Drive
                        match drive_lock.fetch_identity_with_balance(
                            identity_id_bytes,
                            None,
                            sdk.version(),
                        ) {
                            Ok(maybe_partial_identity) => {
                                let partial_identity =
                                    maybe_partial_identity.unwrap_or_else(|| PartialIdentity {
                                        id: identifier,
                                        loaded_public_keys: BTreeMap::new(),
                                        balance: None,
                                        revision: None,
                                        not_found_public_keys: BTreeSet::new(),
                                    });
                                tracing::trace!(
                                    "Fetched identity info for identifier {}: {:?}",
                                    identifier,
                                    partial_identity
                                );
                                partial_identity
                            }
                            Err(e) => {
                                tracing::debug!("Error fetching identity: {:?}", e);
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

                // Create asset lock proofs for all the identity creates and top ups
                let num_start_identities = strategy.start_identities.number_of_identities as u64;
                let num_identity_inserts = (strategy
                    .identity_inserts
                    .frequency
                    .times_per_block_range
                    .start as f64
                    * duration as f64
                    * strategy
                        .identity_inserts
                        .frequency
                        .chance_per_block
                        .unwrap_or(1.0)) as u64;
                let mut num_top_ups: u64 = 0;
                for operation in &strategy.operations {
                    if matches!(operation.op_type, OperationType::IdentityTopUp(_)) {
                        num_top_ups += (operation.frequency.times_per_block_range.start as f64
                            * duration as f64
                            * operation.frequency.chance_per_block.unwrap_or(1.0))
                            as u64;
                    }
                }
                let num_asset_lock_proofs_needed =
                    num_start_identities + num_identity_inserts + num_top_ups;
                let mut asset_lock_proofs: Vec<(AssetLockProof, PrivateKey)> = Vec::new();
                if num_asset_lock_proofs_needed > 0 {
                    let wallet_lock = self.loaded_wallet.lock().await;
                    let num_available_utxos = match wallet_lock
                        .as_ref()
                        .expect("No wallet loaded while getting asset lock proofs")
                    {
                        Wallet::SingleKeyWallet(SingleKeyWallet { utxos, .. }) => utxos.len(),
                    };
                    drop(wallet_lock);
                    if num_available_utxos
                        < num_asset_lock_proofs_needed
                            .try_into()
                            .expect("Couldn't convert num_asset_lock_proofs_needed into usize")
                    {
                        return BackendEvent::StrategyError {
                            error: format!("Not enough UTXOs available in wallet. Available: {}. Need: {}. Go to Wallet screen and create more.", num_available_utxos, num_asset_lock_proofs_needed),
                        };
                    }
                    tracing::info!(
                        "Obtaining {} asset lock proofs for the strategy...",
                        num_asset_lock_proofs_needed
                    );
                    let asset_lock_proof_time = Instant::now();

                    // Broadcast asset locks and receive proofs.
                    let permits = Arc::new(Semaphore::new(20));
                    let starting_balance = strategy.start_identities.starting_balances;
                    let processed = Arc::new(AtomicUsize::new(0));
                    let tasks: FuturesUnordered<_> = (0..num_asset_lock_proofs_needed)
                    .map(|_| {
                        let permits = Arc::clone(&permits);
                        let processed = Arc::clone(&processed);

                        async move {
                            let _permit = permits.acquire_owned().await.ok()?;

                            let mut wallet_lock = self.loaded_wallet.lock().await;
                            let wallet = wallet_lock.as_mut().expect("Wallet not loaded");

                            let (asset_lock_transaction, asset_lock_proof_private_key) = wallet
                                .asset_lock_transaction(None, starting_balance)
                                .map_err(|e| {
                                    tracing::debug!("Error creating asset lock transaction: {:?}", e);
                                    e
                                })
                                .ok()?;

                            let receive_address = wallet.receive_address();
                            drop(wallet_lock);

                            let result;

                                match AppState::broadcast_and_retrieve_asset_lock(sdk, &asset_lock_transaction, &receive_address).await {
                                    Ok(asset_lock_proof) => {
                                        result = Ok((asset_lock_proof, asset_lock_proof_private_key));
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Error broadcasting asset lock transaction and retrieving proof: {:?}",
                                            e
                                        );
                                        result = Err(e);
                                    }
                                }


                            match result {
                                Ok(asset_lock_proof) => {
                                    let prev = processed.fetch_add(1, Ordering::SeqCst);
                                    tracing::trace!(
                                        "Successfully obtained asset lock proof {} of {}",
                                        prev + 1,
                                        num_asset_lock_proofs_needed
                                    );
                                    Some(asset_lock_proof)
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Failed to obtain asset lock proof: {:?}",
                                        e
                                    );
                                    None
                                }
                            }
                        }
                        .boxed_local()
                    })
                    .collect();

                    asset_lock_proofs = join_all(tasks).await.into_iter().flatten().collect();

                    tracing::info!(
                        "Took {} seconds to obtain {} asset lock proofs from {} required",
                        asset_lock_proof_time.elapsed().as_secs(),
                        asset_lock_proofs.len(),
                        num_asset_lock_proofs_needed
                    );
                }

                // Some final initialization
                let mut rng = StdRng::from_entropy(); // Will be passed to state_transitions_for_block
                let mut current_block_info = initial_block_info.clone(); // Used for transition creation and logging
                let mut transition_count: u32 = 0; // Used for logging how many transitions we attempted
                let mut success_count = Arc::new(AtomicU64::new(0)); // Used for logging how many transitions were successful
                let mut load_start_time = Instant::now(); // Time when the load test begins (all blocks after the second block)
                let mut loop_index = 1; // Index of the loop iteration. Represents blocks for block mode and seconds for time mode
                let mut new_identity_ids = Vec::new(); // Will capture the ids of identities added to current_identities
                let mut new_contract_ids = Vec::new(); // Will capture the ids of newly created data contracts
                let broadcast_oks = Arc::new(AtomicU64::new(0)); // Atomic counter for successful broadcasts
                let broadcast_errs = Arc::new(AtomicU64::new(0)); // Atomic counter for failed broadcasts
                let ongoing_broadcasts = Arc::new(AtomicU64::new(0)); // Atomic counter for ongoing broadcasts
                let ongoing_waits = Arc::new(AtomicU64::new(0)); // Atomic counter for ongoing waits
                let wait_oks = Arc::new(AtomicU64::new(0)); // Atomic counter for successful waits
                let wait_errs = Arc::new(AtomicU64::new(0)); // Atomic counter for failed waits
                let mempool_document_counter =
                    Arc::new(Mutex::new(BTreeMap::<(Identifier, Identifier), u64>::new())); // Map to track how many documents an identity has in the mempool per contract
                let hist = Arc::new(Mutex::new(Histogram::<u64>::new(3).unwrap()));

                // Broadcast error counters
                let identity_nonce_error_count = Arc::new(AtomicU64::new(0));
                let insufficient_balance_error_count = Arc::new(AtomicU64::new(0));
                let local_rate_limit_error_count = Arc::new(AtomicU64::new(0));
                let broadcast_connection_error_count = Arc::new(AtomicU64::new(0));

                let broadcast_errors_per_code: Arc<DashMap<Code, AtomicU64>> = Default::default();
                let wait_errors_per_code: Arc<DashMap<Code, AtomicU64>> = Default::default();

                tracing::info!("Initialization complete. Starting strategy execution.");

                // Now loop through the number of blocks or seconds the user asked for, preparing and processing state transitions
                while load_start_time.elapsed().as_secs() < duration || loop_index <= 2 {
                    tracing::debug!("Start loop: {loop_index}");
                    // Every 10 loops, log statistics
                    if loop_index % 10 == 0 {
                        let stats_elapsed = load_start_time.elapsed().as_secs();
                        let stats_attempted = transition_count;
                        let stats_rate = stats_attempted
                            .checked_div(stats_elapsed.try_into().unwrap())
                            .unwrap_or(0);
                        let stats_broadcast_successful = broadcast_oks.load(Ordering::SeqCst);
                        let stats_broadcast_failed = broadcast_errs.load(Ordering::SeqCst);
                        let stats_wait_successful = wait_oks.load(Ordering::SeqCst);
                        let stats_wait_failed = wait_errs.load(Ordering::SeqCst);
                        let stats_ongoing_waits = ongoing_waits.load(Ordering::SeqCst);
                        let stats_ongoing_broadcasts = ongoing_broadcasts.load(Ordering::SeqCst);
                        let hist_lock = hist.lock().await;
                        let stats_p50 = hist_lock.value_at_quantile(0.50) as f64;
                        let stats_p90 = hist_lock.value_at_quantile(0.90) as f64;
                        let stats_p95 = hist_lock.value_at_quantile(0.95) as f64;

                        let mut stats_broadcast_error_messages = Vec::new();
                        for entry in broadcast_errors_per_code.iter() {
                            let code = entry.key();
                            let count = entry.value().load(Ordering::SeqCst);
                            stats_broadcast_error_messages.push(format!("{:?} - {}", code, count));
                        }
                        let mut stats_wait_error_messages = Vec::new();
                        for entry in wait_errors_per_code.iter() {
                            let code = entry.key();
                            let count = entry.value().load(Ordering::SeqCst);
                            stats_wait_error_messages.push(format!("{:?} - {}", code, count));
                        }
                        let broadcast_error_message = if !stats_broadcast_error_messages.is_empty()
                        {
                            format!("{}", stats_broadcast_error_messages.join(", "))
                        } else {
                            String::new()
                        };
                        let wait_error_message = if !stats_wait_error_messages.is_empty() {
                            format!("{}", stats_wait_error_messages.join(", "))
                        } else {
                            String::new()
                        };

                        tracing::info!("\n\n{} secs passed. {} broadcast ({} tx/s)\nBroadcast results: {} successful, {} failed, {} ongoing.\nWait results: {} successful, {} failed, {} ongoing.\nBroadcast errors: {}\nWait errors: {}\nWait times (s): 50% - {} 90% - {} 95% - {}", stats_elapsed, stats_attempted, stats_rate, stats_broadcast_successful, stats_broadcast_failed, stats_ongoing_broadcasts, stats_wait_successful, stats_wait_failed, stats_ongoing_waits, broadcast_error_message, wait_error_message, stats_p50, stats_p90, stats_p95);
                    }

                    let loop_start_time = Instant::now();
                    let broadcast_oks_clone = broadcast_oks.clone();
                    let broadcast_errs_clone = broadcast_errs.clone();
                    let wait_oks_clone = wait_oks.clone();
                    let wait_errs_clone = wait_errs.clone();
                    let ongoing_waits_clone = ongoing_waits.clone();
                    let ongoing_broadcasts_clone = ongoing_broadcasts.clone();
                    let success_count_clone = success_count.clone();
                    let identity_nonce_error_count_clone = identity_nonce_error_count.clone();
                    let insufficient_balance_error_count_clone =
                        insufficient_balance_error_count.clone();
                    let local_rate_limit_error_count_clone = local_rate_limit_error_count.clone();
                    let broadcast_connection_error_count_clone =
                        broadcast_connection_error_count.clone();
                    let broadcast_errors_per_code_clone = broadcast_errors_per_code.clone();
                    let wait_errors_per_code_clone = wait_errors_per_code.clone();

                    // Need to pass self.known_contracts to state_transitions_for_block
                    let mut known_contracts_lock = self.known_contracts.lock().await;

                    let mempool_document_counter_lock = mempool_document_counter.lock().await;
                    let mut current_identities_lock = current_identities.lock().await;

                    // Get the state transitions for the block (or second)
                    let (transitions, finalize_operations, mut new_identities) = strategy
                        .state_transitions_for_block(
                            &mut document_query_callback,
                            &mut identity_fetch_callback,
                            &mut asset_lock_proofs,
                            &current_block_info,
                            &mut current_identities_lock,
                            &mut known_contracts_lock,
                            &mut signer,
                            &mut identity_nonce_counter,
                            &mut contract_nonce_counter,
                            &mempool_document_counter_lock,
                            &mut rng,
                            &StrategyConfig {
                                start_block_height: initial_block_info.height,
                                number_of_blocks: duration,
                            },
                            sdk.version(),
                        );

                    drop(known_contracts_lock);
                    drop(mempool_document_counter_lock);

                    // Add the identities that will be created to current_identities.
                    // Only do this on init block because identity_inserts don't have transfer keys atm
                    // and if we have transfer txs, it will panic if it tries to use one of these identities.
                    // TO-DO: This should be moved to execution after we confirm they were registered.
                    if loop_index < 3 {
                        for identity in &new_identities {
                            new_identity_ids.push(identity.id().to_string(Encoding::Base58))
                        }
                        current_identities_lock.append(&mut new_identities);
                    }

                    // Extra transition type-specific processing
                    for transition in &transitions {
                        match transition {
                            // Add new data contracts to the list of newly created contract ids for logging
                            // Also insert into AppState known_contracts for proof verification
                            // Also insert into local Drive for later getting documents to delete
                            StateTransition::DataContractCreate(contract_create_transition) => {
                                new_contract_ids.extend(
                                    contract_create_transition
                                        .modified_data_ids()
                                        .iter()
                                        .map(|id| id.to_string(Encoding::Base58)),
                                );

                                let mut known_contracts = self.known_contracts.lock().await;
                                let data_contract_serialized =
                                    contract_create_transition.data_contract();
                                let maybe_data_contract = DataContract::try_from_platform_versioned(
                                    data_contract_serialized.clone(),
                                    false,
                                    &mut vec![],
                                    sdk.version(),
                                );
                                match maybe_data_contract {
                                    Ok(contract) => {
                                        known_contracts.insert(
                                            contract_create_transition
                                                .data_contract()
                                                .id()
                                                .to_string(Encoding::Base58),
                                            contract.clone(),
                                        );
                                        let result = drive_lock.apply_contract(
                                            &contract,
                                            current_block_info,
                                            true,
                                            None,
                                            None,
                                            sdk.version(),
                                        );
                                        if let Err(e) = result {
                                            tracing::debug!(
                                                "Failed to add contract to local drive: {e}"
                                            );
                                        }
                                    }
                                    Err(e) => tracing::debug!(
                                        "Failed to convert serialized contract to contract: {e}"
                                    ),
                                }
                            }
                            // Add new documents to the local Drive for later getting documents to delete
                            StateTransition::DocumentsBatch(DocumentsBatchTransition::V0(
                                documents_batch_transition_v0,
                            )) => {
                                for document_transition in
                                    &documents_batch_transition_v0.transitions
                                {
                                    match document_transition {
                                        DocumentTransition::Create(
                                            DocumentCreateTransition::V0(
                                                document_create_transition_v0,
                                            ),
                                        ) => {
                                            let document_type_name = document_create_transition_v0
                                                .base
                                                .document_type_name();
                                            let data_contract_id = document_create_transition_v0
                                                .base
                                                .data_contract_id();
                                            let known_contracts = self.known_contracts.lock().await;
                                            let maybe_data_contract = known_contracts
                                                .get(&data_contract_id.to_string(Encoding::Base58));
                                            match maybe_data_contract {
                                                Some(data_contract) => {
                                                    let maybe_document_type = data_contract
                                                        .document_type_for_name(document_type_name);
                                                    match maybe_document_type {
                                                        Ok(document_type) => {
                                                            let maybe_document = Document::try_from_create_transition_v0(document_create_transition_v0, transition.owner_id(), &current_block_info, &document_type, sdk.version());
                                                            match maybe_document {
                                                                Ok(document) => {
                                                                    let document_info = DocumentInfo::DocumentOwnedInfo((document, None));
                                                                    let owned_document_info =
                                                                        OwnedDocumentInfo {
                                                                            document_info,
                                                                            owner_id: Some(
                                                                                *transition
                                                                                    .owner_id()
                                                                                    .as_bytes(),
                                                                            ),
                                                                        };
                                                                    let result = drive_lock
                                                                        .add_document(
                                                                            owned_document_info,
                                                                            data_contract_id,
                                                                            document_type_name,
                                                                            false,
                                                                            &current_block_info,
                                                                            true,
                                                                            None,
                                                                            sdk.version(),
                                                                        );
                                                                    if let Err(e) = result {
                                                                        tracing::debug!("Failed to add document to local drive: {e}");
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::debug!("Couldn't get document from document create transition: {e}");
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::debug!("Couldn't retrieve document type {} from contract: {}", document_type_name, e)
                                                        }
                                                    }
                                                }
                                                None => {
                                                    tracing::debug!("Data contract {} not found in known_contracts", data_contract_id.to_string(Encoding::Base58));
                                                    continue;
                                                }
                                            }
                                        }
                                        _ => {
                                            // nothing
                                        }
                                    }
                                }
                            }
                            _ => {
                                // nothing
                            }
                        };
                    }

                    // Process each FinalizeBlockOperation, which so far is just adding keys to identities
                    for operation in finalize_operations {
                        match operation {
                            FinalizeBlockOperation::IdentityAddKeys(identifier, keys) => {
                                if let Some(identity) = current_identities_lock
                                    .iter_mut()
                                    .find(|id| id.id() == identifier)
                                {
                                    for key in keys {
                                        identity.add_public_key(key);
                                    }
                                }
                            }
                        }
                    }

                    // Update the loaded_identity_clone and loaded_identity_lock with the latest state of the identity
                    if let Some(modified_identity) = current_identities_lock
                        .iter()
                        .find(|identity| identity.id() == loaded_identity_clone.id())
                    {
                        loaded_identity_clone = modified_identity.clone();
                        *loaded_identity_lock = modified_identity.clone();
                    }

                    drop(current_identities_lock);

                    // Now process the state transitions
                    if !transitions.is_empty() {
                        tracing::info!(
                            "Prepared {} state transitions for loop {}",
                            transitions.len(),
                            loop_index
                        );

                        // A queue for the state transitions for the block (or second)
                        let st_queue: VecDeque<StateTransition> = transitions.clone().into();

                        // We will concurrently broadcast the state transitions, so collect the futures
                        let mut broadcast_futures = Vec::new();

                        for transition in st_queue.iter() {
                            transition_count += 1; // Used for logging how many transitions we attempted
                            let transition_clone = transition.clone();
                            let transition_id = hex::encode(
                                transition
                                    .transaction_id()
                                    .expect("Expected transaction to serialize"),
                            )
                            .to_string()
                            .reverse();
                            let mempool_document_counter_clone = mempool_document_counter.clone();
                            let current_identities_clone = Arc::clone(&current_identities);

                            let broadcast_oks = broadcast_oks_clone.clone();
                            let broadcast_errs = broadcast_errs_clone.clone();
                            let ongoing_broadcasts = ongoing_broadcasts_clone.clone();
                            let success_count = success_count_clone.clone();
                            let identity_nonce_error_count =
                                identity_nonce_error_count_clone.clone();
                            let insufficient_balance_error_count =
                                insufficient_balance_error_count_clone.clone();
                            let local_rate_limit_error_count =
                                local_rate_limit_error_count_clone.clone();
                            let broadcast_connection_error_count =
                                broadcast_connection_error_count_clone.clone();
                            let broadcast_errors_per_code = broadcast_errors_per_code_clone.clone();

                            let mut request_settings = RequestSettings::default();
                            // Time-based strategy body
                            if loop_index != 1 && loop_index != 2 {
                                // time mode loading
                                request_settings.connect_timeout = Some(Duration::from_millis(
                                    min(seconds_per_loop * 1000 / 2, 3000),
                                ));
                                request_settings.timeout = Some(Duration::from_millis(min(
                                    seconds_per_loop * 1000 / 2,
                                    3000,
                                )));
                                request_settings.retries = Some(0);
                            }

                            // Prepare futures for broadcasting transitions
                            let future = async move {
                                match transition_clone.broadcast_request_for_state_transition() {
                                    Ok(broadcast_request) => {
                                        ongoing_broadcasts.fetch_add(1, Ordering::SeqCst);
                                        let broadcast_result = broadcast_request.execute(&sdk.clone(), request_settings).await;
                                        ongoing_broadcasts.fetch_sub(1, Ordering::SeqCst);
                                        match broadcast_result {
                                            Ok(_) => {
                                                broadcast_oks.fetch_add(1, Ordering::SeqCst);
                                                success_count.fetch_add(1, Ordering::SeqCst);
                                                let transition_owner_id = transition_clone.owner_id().to_string(Encoding::Base58);
                                                if loop_index != 1 && loop_index != 2 {
                                                    tracing::info!("Successfully broadcasted transition: {}. ID: {}. Owner ID: {:?}", transition_clone.name(), transition_id, transition_owner_id);
                                                }
                                                if transition_clone.name() == "DocumentsBatch" {
                                                    let contract_ids = match transition_clone.clone() {
                                                        StateTransition::DocumentsBatch(DocumentsBatchTransition::V0(transition)) => transition.transitions.iter().map(|document_transition|
                                                            match document_transition {
                                                                DocumentTransition::Create(DocumentCreateTransition::V0(create_tx)) => create_tx.base.data_contract_id(),
                                                                DocumentTransition::Delete(DocumentDeleteTransition::V0(delete_tx)) => delete_tx.base.data_contract_id(),
                                                                _ => panic!("This should never happen")
                                                            }
                                                        ).collect_vec(),
                                                        _ => panic!("This shouldn't happen")
                                                    };
                                                    for contract_id in contract_ids {
                                                        let mut mempool_document_counter_clone_lock = mempool_document_counter_clone.lock().await;
                                                        let count = mempool_document_counter_clone_lock.entry((transition_clone.owner_id(), contract_id)).or_insert(0);
                                                        *count += 1;
                                                        // tracing::trace!(" + Incremented identity {} tx counter for contract {}. Count: {}", transition_owner_id, contract_id.to_string(Encoding::Base58), count);
                                                    }
                                                }
                                                Ok((transition_clone, broadcast_result))
                                            },
                                            Err(e) => {
                                                match e.inner {
                                                    rs_dapi_client::DapiClientError::Transport(ref e, ..) => {broadcast_errors_per_code
                                                        .entry(match e {
                                                            rs_dapi_client::transport::TransportError::Grpc(status) => status.code(),
                                                        })
                                                        .or_insert_with(|| AtomicU64::new(0))
                                                        .fetch_add(1, Ordering::SeqCst);},
                                                    _ => {
                                                        broadcast_errors_per_code
                                                            .entry(Code::Unknown)
                                                            .or_insert_with(|| AtomicU64::new(0))
                                                            .fetch_add(1, Ordering::SeqCst);
                                                    }
                                                };
                                                broadcast_errs.fetch_add(1, Ordering::SeqCst);
                                                tracing::error!("Error: Failed to broadcast {} transition: {:?}. ID: {}", transition_clone.name(), e, transition_id);
                                                if e.to_string().contains("Insufficient identity") {
                                                    insufficient_balance_error_count.fetch_add(1, Ordering::SeqCst);
                                                    // Top up. This logic works but it slows the broadcasting down slightly.
                                                    if top_up_amount > 0 {
                                                        let current_identities = Arc::clone(&current_identities_clone);
                                                        let sdk_clone = sdk.clone();
                                                        let (tx, rx) = oneshot::channel();

                                                        // Lock the wallet and clone the necessary data before moving into the async block
                                                        let asset_lock_transaction;
                                                        let asset_lock_proof_private_key;
                                                        let wallet_receive_address;
                                                        {
                                                            let mut wallet_lock = self.loaded_wallet.lock().await;
                                                            let wallet = wallet_lock.as_mut().unwrap();
                                                            let (asset_lock_tx, asset_lock_proof_key) = wallet.asset_lock_transaction(None, top_up_amount).unwrap();
                                                            asset_lock_transaction = asset_lock_tx.clone();
                                                            asset_lock_proof_private_key = asset_lock_proof_key.clone();
                                                            wallet_receive_address = wallet.receive_address();
                                                        }

                                                        // Spawn a blocking task for the top-up process
                                                        tokio::task::spawn_blocking(move || {
                                                            // Use block_in_place to run async code within blocking context
                                                            let result = tokio::task::block_in_place(|| {
                                                                tokio::runtime::Handle::current().block_on(async {
                                                                    let mut current_identities = current_identities.lock().await;

                                                                    // Top up
                                                                    match try_broadcast_and_retrieve_asset_lock(&sdk_clone, &asset_lock_transaction, &wallet_receive_address, 2).await {
                                                                        Ok(asset_lock_proof) => {
                                                                            tracing::trace!("Successfully obtained asset lock proof for top up");
                                                                            let identity = current_identities.iter_mut().find(|identity| identity.id() == transition_clone.owner_id()).expect("Expected to find identity ID matching transition owner ID");

                                                                            let state_transition = IdentityTopUpTransition::try_from_identity(
                                                                                identity,
                                                                                asset_lock_proof,
                                                                                asset_lock_proof_private_key.inner.as_ref(),
                                                                                0,
                                                                                sdk_clone.version(),
                                                                                None,
                                                                            ).expect("Expected to make a top up transition");

                                                                            let request = state_transition.broadcast_request_for_state_transition().expect("Expected to create broadcast request for top up");

                                                                            match request
                                                                                .clone()
                                                                                .execute(&sdk_clone, RequestSettings::default())
                                                                                .await {
                                                                                    Ok(_) => tracing::trace!("Successfully topped up identity"),
                                                                                    Err(e) => tracing::debug!("Failed to top up identity: {:?}", e)
                                                                                };
                                                                        }
                                                                        Err(_) => {
                                                                            tracing::debug!("Failed to obtain asset lock proof for top up");
                                                                        }
                                                                    }
                                                                })
                                                            });

                                                            // Send the result back to the main async context
                                                            tx.send(result).unwrap();
                                                        });

                                                        // Await the result of the blocking task
                                                        let _ = rx.await;

                                                    }
                                                } else if e.to_string().contains("invalid identity nonce") {
                                                    identity_nonce_error_count.fetch_add(1, Ordering::SeqCst);
                                                } else if e.to_string().contains("rate-limit") {
                                                    local_rate_limit_error_count.fetch_add(1, Ordering::SeqCst);
                                                } else if e.to_string().contains("error trying to connect") {
                                                    broadcast_connection_error_count.fetch_add(1, Ordering::SeqCst);
                                                }
                                                Err(e)
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        broadcast_errs.fetch_add(1, Ordering::SeqCst);
                                        tracing::debug!("Error preparing broadcast request for transition: {}, Error: {:?}", transition_clone.name(), e);
                                        Err(e)
                                    }.expect("Expected to prepare broadcast for request for state transition")
                                }
                            };

                            broadcast_futures.push(future);
                        }

                        // Concurrently execute all broadcast requests for independent transitions
                        let broadcast_results = join_all(broadcast_futures).await;

                        // Now wait for results

                        // If we're in block mode, or index 1 or 2 of time mode
                        if loop_index == 1 || loop_index == 2 {
                            let request_settings = RequestSettings {
                                connect_timeout: None,
                                timeout: Some(Duration::from_secs(75)),
                                retries: Some(5),
                                ban_failed_address: Some(false),
                            };

                            let mut wait_futures = Vec::new();
                            for (tx_index, result) in broadcast_results.into_iter().enumerate() {
                                match result {
                                    Ok((transition, broadcast_result)) => {
                                        let transition_type = transition.name().to_owned();
                                        let transition_id = hex::encode(
                                            transition
                                                .transaction_id()
                                                .expect("Expected transaction to serialize"),
                                        )
                                        .to_string()
                                        .reverse();

                                        if broadcast_result.is_err() {
                                            tracing::debug!(
                                                "Error broadcasting state transition {} ({}) for loop {}: {:?}. ID: {}",
                                                tx_index + 1,
                                                transition_type,
                                                loop_index,
                                                broadcast_result.err().unwrap(),
                                                transition_id
                                            );
                                            continue;
                                        }

                                        // I think what's happening here is we're using this data_contract_clone for proof verification later
                                        let data_contract_id_option = match &transition {
                                            StateTransition::DocumentsBatch(
                                                DocumentsBatchTransition::V0(documents_batch),
                                            ) => {
                                                documents_batch.transitions.get(0).and_then(
                                                    |document_transition| {
                                                        match document_transition {
                                                            DocumentTransition::Create(
                                                                DocumentCreateTransition::V0(
                                                                    create_transition,
                                                                ),
                                                            ) => Some(
                                                                create_transition
                                                                    .base
                                                                    .data_contract_id(),
                                                            ),
                                                            // Add handling for Replace and Delete transitions if necessary
                                                            _ => None,
                                                        }
                                                    },
                                                )
                                            }
                                            // Handle other state transition types that involve data contracts here
                                            _ => None,
                                        };
                                        let known_contracts_lock =
                                            self.known_contracts.lock().await;
                                        let data_contract_clone = if let Some(data_contract_id) =
                                            data_contract_id_option
                                        {
                                            let data_contract_id_str =
                                                data_contract_id.to_string(Encoding::Base58);
                                            known_contracts_lock.get(&data_contract_id_str).cloned()
                                        } else {
                                            None
                                        };
                                        drop(known_contracts_lock);

                                        let wait_future = async move {
                                            let wait_result = match transition
                                                .wait_for_state_transition_result_request()
                                            {
                                                Ok(wait_request) => {
                                                    wait_request
                                                        .execute(sdk, request_settings)
                                                        .await
                                                }
                                                Err(e) => {
                                                    tracing::debug!(
                                                        "Error creating wait request for state transition {} loop {}: {:?}. ID: {}",
                                                        tx_index + 1, loop_index, e, transition_id
                                                    );
                                                    return None;
                                                }
                                            };

                                            match wait_result {
                                                Ok(wait_response) => {
                                                    Some(if let Some(wait_for_state_transition_result_response::Version::V0(v0_response)) = &wait_response.inner.version {
                                                        if let Some(metadata) = &v0_response.metadata {
                                                            // Verification of the proof
                                                            if let Some(wait_for_state_transition_result_response_v0::Result::Proof(proof)) = &v0_response.result {
                                                                if verify_proofs {
                                                                    let epoch = Epoch::new(metadata.epoch as u16).expect("Expected to get epoch from metadata in proof verification");
                                                                    // For proof verification, if it's a DocumentsBatch, include the data contract, else don't
                                                                    let verified = if transition.name() == "DocumentsBatch" {
                                                                        match data_contract_clone.as_ref() {
                                                                            Some(data_contract) => {
                                                                                Drive::verify_state_transition_was_executed_with_proof(
                                                                                    &transition,
                                                                                    &BlockInfo {
                                                                                        time_ms: metadata.time_ms,
                                                                                        height: metadata.height,
                                                                                        core_height: metadata.core_chain_locked_height,
                                                                                        epoch,
                                                                                    },
                                                                                    proof.grovedb_proof.as_slice(),
                                                                                    &|_| Ok(Some(data_contract.clone().into())),
                                                                                    sdk.version(),
                                                                                )
                                                                            }
                                                                            None => Err(drive::error::Error::Proof(ProofError::UnknownContract("Data contract ID not found in known_contracts".into()))),
                                                                        }
                                                                    } else {
                                                                        Drive::verify_state_transition_was_executed_with_proof(
                                                                            &transition,
                                                                            &BlockInfo {
                                                                                time_ms: metadata.time_ms,
                                                                                height: metadata.height,
                                                                                core_height: metadata.core_chain_locked_height,
                                                                                epoch,
                                                                            },
                                                                            proof.grovedb_proof.as_slice(),
                                                                            &|_| Ok(None),
                                                                            sdk.version(),
                                                                        )
                                                                    };

                                                                    match verified {
                                                                        Ok(_) => {
                                                                            tracing::trace!("Successfully processed and verified proof for state transition {} ({}), second {} (Actual block height: {}). ID: {}", tx_index + 1, transition_type, tx_index, metadata.height, transition_id);
                                                                        }
                                                                        Err(e) => tracing::debug!("Error verifying state transition execution proof: {}", e),
                                                                    }
                                                                } else {
                                                                    tracing::trace!(
                                                                        "Successfully broadcasted and processed state transition {} ({}) for loop {} (Actual block height: {}). ID: {}",
                                                                        tx_index + 1, transition.name(), loop_index, metadata.height, transition_id
                                                                    );
                                                                }
                                                            } else if let Some(wait_for_state_transition_result_response_v0::Result::Error(e)) = &v0_response.result {
                                                                tracing::debug!("Transition failed in mempool with error: {:?}. ID: {}", e, transition_id);
                                                            } else {
                                                                tracing::trace!("Received empty response for transition with ID: {}", transition_id);
                                                            }

                                                            // Log the Base58 encoded IDs of any created contracts or identities
                                                            // Also add data contracts to known contracts. To be removed at the end of strategy run.
                                                            match transition.clone() {
                                                                StateTransition::IdentityCreate(identity_create_transition) => {
                                                                    let ids = identity_create_transition.modified_data_ids();
                                                                    for id in ids {
                                                                        let encoded_id: String = id.to_string(Encoding::Base58);
                                                                        tracing::trace!("Created identity: {}", encoded_id);
                                                                    }
                                                                },
                                                                StateTransition::DataContractCreate(contract_create_transition) => {
                                                                    let ids = contract_create_transition.modified_data_ids();
                                                                    for id in ids {
                                                                        let encoded_id: String = id.to_string(Encoding::Base58);
                                                                        tracing::trace!("Created contract: {}", encoded_id);                                                                        
                                                                    }
                                                                },
                                                                _ => {
                                                                    // nothing
                                                                }
                                                            }
                                                        } else {
                                                            tracing::trace!("Broadcasted transition and received response but no metadata");
                                                        }
                                                    })
                                                }
                                                Err(e) => {
                                                    tracing::debug!("Wait result error: {:?}", e);
                                                    None
                                                }
                                            }
                                        };
                                        wait_futures.push(wait_future);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Error preparing broadcast request for state transition {} second {}: {:?}",
                                            tx_index + 1,
                                            current_block_info.height,
                                            e
                                        );
                                    }
                                }
                            }

                            // Wait for all state transition result futures to complete
                            let _wait_results = join_all(wait_futures).await;
                        } else {
                            // Time mode when index is greater than 2
                            let request_settings = RequestSettings {
                                connect_timeout: None,
                                timeout: Some(Duration::from_secs(75)),
                                retries: Some(5),
                                ban_failed_address: Some(false),
                            };

                            let sdk_clone = sdk.clone();
                            for (tx_index, result) in broadcast_results.into_iter().enumerate() {
                                let wait_oks = wait_oks_clone.clone();
                                let wait_errs = wait_errs_clone.clone();
                                let ongoing_waits = ongoing_waits_clone.clone();
                                let wait_errors_per_code = wait_errors_per_code_clone.clone();

                                let mempool_document_counter_clone =
                                    mempool_document_counter.clone();
                                match result {
                                    Ok((transition, _broadcast_result)) => {
                                        let transition_type = transition.name().to_owned();
                                        let transition_id = hex::encode(
                                            transition
                                                .transaction_id()
                                                .expect("Expected transaction to serialize"),
                                        )
                                        .to_string()
                                        .reverse();
                                        let sdk_clone_inner = sdk_clone.clone();

                                        tokio::spawn({
                                            let hist = Arc::clone(&hist);

                                            async move {
                                                if let Ok(wait_request) = transition
                                                    .wait_for_state_transition_result_request()
                                                {
                                                    ongoing_waits.fetch_add(1, Ordering::SeqCst);
                                                    let wait_start_time = Instant::now();
                                                    match wait_request
                                                        .execute(&sdk_clone_inner, request_settings)
                                                        .await
                                                    {
                                                        Ok(wait_response) => {
                                                            let wait_time =
                                                                wait_start_time.elapsed().as_secs();
                                                            let mut hist_lock = hist.lock().await;
                                                            hist_lock.record(wait_time).unwrap();

                                                            if let Some(wait_for_state_transition_result_response::Version::V0(v0_response)) = &wait_response.inner.version {
                                                            if let Some(wait_for_state_transition_result_response_v0::Result::Proof(_proof)) = &v0_response.result {
                                                                // Assume the proof is correct in time mode for now
                                                                // Decrement the transitions counter
                                                                wait_oks.fetch_add(1, Ordering::SeqCst);
                                                                ongoing_waits.fetch_sub(1, Ordering::SeqCst);
                                                                tracing::trace!(" >>> Transition was included in a block. ID: {}", transition_id);
                                                                if transition_type == "DocumentsBatch" {
                                                                    let contract_ids = match transition.clone() {
                                                                        StateTransition::DocumentsBatch(DocumentsBatchTransition::V0(transition)) => transition.transitions.iter().map(|document_transition|
                                                                            match document_transition {
                                                                                DocumentTransition::Create(DocumentCreateTransition::V0(create_tx)) => create_tx.base.data_contract_id(),
                                                                                DocumentTransition::Delete(DocumentDeleteTransition::V0(delete_tx)) => delete_tx.base.data_contract_id(),
                                                                                _ => panic!("This should never happen")
                                                                            }
                                                                        ).collect_vec(),
                                                                        _ => panic!("This shouldn't happen")
                                                                    };
                                                                    for contract_id in contract_ids {
                                                                        let mut mempool_document_counter_lock = mempool_document_counter_clone.lock().await;
                                                                        let count = mempool_document_counter_lock.entry((transition.owner_id(), contract_id)).or_insert(0);
                                                                        *count -= 1;
                                                                        // tracing::trace!(" - Decremented identity {} tx counter for contract {}. Count: {}", transition_owner_id, contract_id.to_string(Encoding::Base58), count);
                                                                    }
                                                                }
                                                            } else if let Some(wait_for_state_transition_result_response_v0::Result::Error(e)) = &v0_response.result {
                                                                wait_errs.fetch_add(1, Ordering::SeqCst);
                                                                ongoing_waits.fetch_sub(1, Ordering::SeqCst);
                                                                tracing::debug!(" >>> Transition failed in mempool with error: {:?}. ID: {}", e, transition_id);
                                                                if transition_type == "DocumentsBatch" {
                                                                    let contract_ids = match transition.clone() {
                                                                        StateTransition::DocumentsBatch(DocumentsBatchTransition::V0(transition)) => transition.transitions.iter().map(|document_transition|
                                                                            match document_transition {
                                                                                DocumentTransition::Create(DocumentCreateTransition::V0(create_tx)) => create_tx.base.data_contract_id(),
                                                                                DocumentTransition::Delete(DocumentDeleteTransition::V0(delete_tx)) => delete_tx.base.data_contract_id(),
                                                                                _ => panic!("This should never happen")
                                                                            }
                                                                        ).collect_vec(),
                                                                        _ => panic!("This shouldn't happen")
                                                                    };
                                                                    for contract_id in contract_ids {
                                                                        let mut mempool_document_counter_lock = mempool_document_counter_clone.lock().await;
                                                                        let count = mempool_document_counter_lock.entry((transition.owner_id(), contract_id)).or_insert(0);
                                                                        *count -= 1;
                                                                        // tracing::trace!(" - Decremented identity {} tx counter for contract {}. Count: {}", transition_owner_id, contract_id.to_string(Encoding::Base58), count);
                                                                    }
                                                                }
                                                            } else {
                                                                wait_oks.fetch_add(1, Ordering::SeqCst);
                                                                ongoing_waits.fetch_sub(1, Ordering::SeqCst);
                                                                tracing::trace!(" >>> Received empty response for transition with ID: {}", transition_id);
                                                            }
                                                        }
                                                        }
                                                        Err(e) => {
                                                            wait_errs
                                                                .fetch_add(1, Ordering::SeqCst);
                                                            ongoing_waits
                                                                .fetch_sub(1, Ordering::SeqCst);
                                                            match e.inner {
                                                                rs_dapi_client::DapiClientError::Transport(ref e, ..) => {wait_errors_per_code
                                                                    .entry(match e {
                                                                        rs_dapi_client::transport::TransportError::Grpc(status) => status.code(),
                                                                    })
                                                                    .or_insert_with(|| AtomicU64::new(0))
                                                                    .fetch_add(1, Ordering::SeqCst);},
                                                                _ => {
                                                                    wait_errors_per_code
                                                                    .entry(Code::Unknown)
                                                                    .or_insert_with(|| AtomicU64::new(0))
                                                                    .fetch_add(1, Ordering::SeqCst);
                                                                    }
                                                            };
                                                            tracing::debug!("Error waiting for state transition result: {:?}, {}, Tx ID: {}", e, transition_type, transition_id);
                                                        }
                                                    }
                                                } else {
                                                    tracing::debug!("Failed to create wait request for state transition {}: {}", tx_index + 1, transition_type);
                                                }
                                            }
                                        });
                                    }
                                    Err(_) => {
                                        // This is already logged
                                    }
                                }
                            }
                        }

                        // Reset the load_start_time
                        // Also, sleep 10 seconds to let nodes update state
                        if loop_index == 1 || loop_index == 2 {
                            tracing::trace!(
                                "Sleep 10 seconds to allow initialization transactions to process"
                            );
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            load_start_time = Instant::now();
                        }
                    } else {
                        // No transitions prepared for the block (or second)
                        tracing::trace!("Prepared 0 state transitions for loop {}", loop_index);
                    }

                    if loop_index == 2 {
                        init_time = init_start_time.elapsed();
                    }

                    // Update current_block_info and index for next loop iteration
                    current_block_info.height += 1;
                    let current_time_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("time went backwards")
                        .as_millis();
                    current_block_info.time_ms = current_time_ms as u64;
                    loop_index += 1;

                    // Make sure the loop doesn't iterate faster than once per seconds_per_loop in time mode
                    let elapsed = loop_start_time.elapsed();
                    if elapsed < Duration::from_secs(1) {
                        let remaining_time = Duration::from_secs(seconds_per_loop) - elapsed;
                        tokio::time::sleep(remaining_time).await;
                    }
                }

                // Strategy execution is finished
                tracing::info!("-----Strategy '{}' finished running-----", strategy_name);

                // Log oks and errs
                tracing::info!(
                    "Successfully broadcasted: {}, Failed to broadcast: {}",
                    broadcast_oks.load(Ordering::SeqCst),
                    broadcast_errs.load(Ordering::SeqCst)
                );

                // Time the execution took
                let load_execution_run_time = load_start_time.elapsed();

                tracing::info!(
                    "Strategy execution ran for {} seconds and intended to run for {} seconds.",
                    load_execution_run_time.as_secs(),
                    duration
                );

                // Log all the newly created identities and contracts.
                // Note these txs were not confirmed. They were just attempted at least.
                tracing::info!(
                    "Newly created identities (attempted): {:?}",
                    new_identity_ids
                );
                tracing::info!(
                    "Newly created contracts (attempted): {:?}",
                    new_contract_ids
                );

                // Withdraw all funds from newly created identities back to the wallet
                let mut current_identities = current_identities.lock().await;
                if current_identities.len() > 0 {
                    current_identities.remove(0); // Remove loaded identity from the vector
                }
                let wallet_lock = self
                    .loaded_wallet
                    .lock()
                    .await
                    .clone()
                    .expect("Expected a loaded wallet while withdrawing");
                tracing::info!("Withdrawing funds from newly created identities back to the loaded wallet (if they have transfer keys)...");
                let mut withdrawals_count = 0;
                for identity in current_identities.clone() {
                    if identity
                        .get_first_public_key_matching(
                            Purpose::TRANSFER,
                            [SecurityLevel::CRITICAL].into(),
                            KeyType::all_key_types().into(),
                            false,
                        )
                        .is_some()
                    {
                        let result = identity
                            .withdraw(
                                sdk,
                                Some(wallet_lock.receive_address()),
                                identity.balance() - 1_000_000, // not sure what this should be
                                None,
                                None,
                                signer.clone(),
                                None,
                            )
                            .await;
                        match result {
                            Ok(balance) => {
                                tracing::info!(
                                    "Withdrew {} from identity {}",
                                    balance,
                                    identity.id().to_string(Encoding::Base58)
                                );
                                withdrawals_count += 1;
                            }
                            Err(e) => {
                                if e.to_string().contains("invalid proof") {
                                    tracing::info!(
                                        "Withdrew from identity {} but proof not verified",
                                        identity.id().to_string(Encoding::Base58)
                                    );
                                    withdrawals_count += 1;
                                } else {
                                    tracing::debug!(
                                        "Error withdrawing from identity {}: {}",
                                        identity.id().to_string(Encoding::Base58),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
                tracing::info!("Completed {} withdrawals.", withdrawals_count);
                drop(wallet_lock);

                // Refresh the identity at the end
                drop(loaded_identity_lock);
                let refresh_result = self.refresh_loaded_identity(&sdk).await;
                if let Err(ref e) = refresh_result {
                    tracing::warn!("Failed to refresh identity after running strategy: {:?}", e);
                }

                // Attempt to retrieve the final balance from the refreshed identity
                let final_balance_identity = match refresh_result {
                    Ok(refreshed_identity_lock) => {
                        // Successfully refreshed, now access the balance
                        refreshed_identity_lock.balance()
                    }
                    Err(_) => initial_balance_identity,
                };

                let dash_spent_identity = (initial_balance_identity as f64
                    - final_balance_identity as f64)
                    / 100_000_000_000.0;

                let loaded_wallet_lock = self.loaded_wallet.lock().await;
                let final_balance_wallet = loaded_wallet_lock.clone().unwrap().balance();
                let dash_spent_wallet = (initial_balance_wallet as f64
                    - final_balance_wallet as f64)
                    / 100_000_000_000.0;

                // For time mode, success_count is just the number of broadcasts
                success_count = broadcast_oks;

                // Make sure we don't divide by 0 when we determine the tx/s rate
                let mut load_run_time = 1;
                if (load_execution_run_time.as_secs()) > 1 {
                    load_run_time = load_execution_run_time.as_secs();
                }

                // Calculate transactions per second
                let mut tps: f32 = 0.0;
                if transition_count
                    > (strategy.start_contracts.len() as u32
                        + strategy.start_identities.number_of_identities as u32)
                {
                    tps = ((transition_count
                        - strategy.start_contracts.len() as u32
                        - strategy.start_identities.number_of_identities as u32)
                        as u64
                        / (load_run_time)) as f32
                };
                let mut successful_tps: f32 = 0.0;
                if success_count.load(Ordering::SeqCst)
                    > (strategy.start_contracts.len() as u64
                        + strategy.start_identities.number_of_identities as u64)
                {
                    successful_tps = ((success_count.load(Ordering::SeqCst)
                        - strategy.start_contracts.len() as u64
                        - strategy.start_identities.number_of_identities as u64)
                        as u64
                        / (load_run_time)) as f32
                };
                let mut success_percent = 0;
                if success_count.load(Ordering::SeqCst)
                    > (strategy.start_contracts.len() as u64
                        + strategy.start_identities.number_of_identities as u64)
                {
                    success_percent = (((success_count.load(Ordering::SeqCst)
                        - strategy.start_contracts.len() as u64
                        - strategy.start_identities.number_of_identities as u64)
                        as f64
                        / (transition_count
                            - strategy.start_contracts.len() as u32
                            - strategy.start_identities.number_of_identities as u32)
                            as f64)
                        * 100.0) as u64
                };

                // Clear self.supporting_contracts
                let mut supporting_contracts_lock = self.supporting_contracts.lock().await;
                supporting_contracts_lock.clear();

                // Time mode
                tracing::info!(
                        "-----Strategy '{}' completed-----\n\nBroadcasts attempted: {}\nBroadcasts succeeded: {}\nNumber of loops: {}\nLoad run time: \
                        {:?} seconds\nInit run time: {} seconds\nAttempted rate (approx): {} txs/s\nSuccessful rate: {} tx/s\nSuccess percentage: {}%\nDash spent (Loaded Identity): {}\nDash spent (Wallet): {}\nBroadcast nonce \
                        errors: {}\nBroadcast balance errors: {}\nBroadcast rate limit errors: {}\nBroadcast connection errors: {}",
                        strategy_name,
                        transition_count,
                        success_count.load(Ordering::SeqCst),
                        loop_index-3, // Minus 3 because we still incremented one at the end of the last loop, and don't count the first two blocks
                        load_run_time,
                        init_time.as_secs(),
                        tps,
                        successful_tps,
                        success_percent,
                        dash_spent_identity,
                        dash_spent_wallet,
                        identity_nonce_error_count.load(Ordering::SeqCst),
                        insufficient_balance_error_count.load(Ordering::SeqCst),
                        local_rate_limit_error_count.load(Ordering::SeqCst),
                        broadcast_connection_error_count.load(Ordering::SeqCst)
                    );

                BackendEvent::StrategyCompleted {
                    strategy_name: strategy_name.clone(),
                    result: StrategyCompletionResult::Success {
                        block_mode: false,
                        final_block_height: current_block_info.height,
                        start_block_height: initial_block_info.height,
                        success_count: success_count.load(Ordering::SeqCst) as u64,
                        transition_count: transition_count.try_into().unwrap(),
                        rate: tps,
                        success_rate: successful_tps,
                        success_percent: success_percent,
                        run_time: load_execution_run_time,
                        init_time: init_time,
                        dash_spent_identity,
                        dash_spent_wallet,
                    },
                }
            }
            StrategyTask::RemoveLastContract(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;

                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    // Remove the last contract_with_update entry from the strategy
                    strategy.start_contracts.pop();

                    // Also remove the corresponding entry from the displayed contracts
                    if let Some(contract_names) = contract_names_lock.get_mut(&strategy_name) {
                        // Assuming each entry in contract_names corresponds to an entry in
                        // start_contracts
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
                    BackendEvent::StrategyError {
                        error: "Strategy doesn't exist in app state".to_string(),
                    }
                }
            }
            StrategyTask::ClearContracts(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let mut contract_names_lock = self.available_strategies_contract_names.lock().await;

                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    // Clear contract_with_updates for the strategy
                    strategy.start_contracts.clear();

                    // Also clear the displayed contracts
                    if let Some(contract_names) = contract_names_lock.get_mut(&strategy_name) {
                        contract_names.clear();
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
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state"),
                    }
                }
            }
            StrategyTask::ClearOperations(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                let contract_names_lock = self.available_strategies_contract_names.lock().await;

                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    // Clear operations for the strategy
                    strategy.operations.clear();

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
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state"),
                    }
                }
            }
            StrategyTask::RemoveIdentityInserts(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    strategy.identity_inserts = IdentityInsertInfo::default();
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: format!("Strategy doesn't exist in app state"),
                    }
                }
            }
            StrategyTask::RemoveStartIdentities(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    strategy.start_identities = StartIdentities::default();
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: "Strategy doesn't exist in app state".to_string(),
                    }
                }
            }
            StrategyTask::RemoveLastOperation(strategy_name) => {
                let mut strategies_lock = self.available_strategies.lock().await;
                if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                    strategy.operations.pop();
                    BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(&strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            self.available_strategies_contract_names.lock().await,
                            |names| names.get_mut(&strategy_name).expect("inconsistent data"),
                        ),
                    ))
                } else {
                    BackendEvent::StrategyError {
                        error: "Strategy doesn't exist in app state".to_string(),
                    }
                }
            }
        }
    }

    async fn update_known_contracts(
        &self,
        sdk: &Sdk,
        known_contracts: &Mutex<KnownContractsMap>,
    ) -> Result<(), String> {
        let contract_ids = {
            let contracts_lock = known_contracts.lock().await;
            contracts_lock.keys().cloned().collect::<Vec<String>>()
        };

        // Clear known contracts first. This is necessary for some reason, I don't know.
        let mut contracts_lock = known_contracts.lock().await;
        contracts_lock.clear();
        drop(contracts_lock);

        for contract_id_str in contract_ids.iter() {
            let contract_id = match Identifier::from_string(contract_id_str, Encoding::Base58) {
                Ok(id) => id,
                Err(e) => match contract_id_str.as_str() {
                    "DPNS" => dpns_contract::ID,
                    "Dashpay" => dashpay_contract::ID,
                    _ => return Err(format!("Failed to update known contracts: {e}")),
                },
            };

            match DataContract::fetch(&*sdk, contract_id).await {
                Ok(Some(data_contract)) => {
                    let mut contracts_lock = known_contracts.lock().await;
                    contracts_lock.insert(contract_id_str.clone(), data_contract);
                }
                Ok(None) => {
                    tracing::debug!("Contract not found for ID: {}", contract_id_str);
                }
                Err(e) => {
                    return Err(format!(
                        "Error fetching contract {}: {}",
                        contract_id_str, e
                    ));
                }
            }
        }

        Ok(())
    }
}

async fn try_broadcast_and_retrieve_asset_lock(
    sdk: &Sdk,
    asset_lock_transaction: &Transaction,
    receive_address: &Address,
    retries: usize,
) -> Result<AssetLockProof, ()> {
    for attempt in 0..=retries {
        match AppState::broadcast_and_retrieve_asset_lock(
            sdk,
            asset_lock_transaction,
            receive_address,
        )
        .await
        {
            Ok(asset_lock_proof) => {
                return Ok(asset_lock_proof);
            }
            Err(_) => {
                tracing::debug!(
                    "Failed to obtain asset lock proof on attempt {}",
                    attempt + 1
                );
                if attempt < retries {
                    tracing::info!(
                        "Retrying to obtain asset lock proof (attempt {})",
                        attempt + 2
                    );
                }
            }
        }
    }
    Err(())
}
