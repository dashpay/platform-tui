//! Screens and forms related to strategies manipulation.

use std::collections::BTreeMap;

use dash_platform_sdk::platform::DataContract;
use dpp::{tests::json_document::json_document_to_created_contract, version::PlatformVersion};
use strategy_tests::{
    operations::{
        DataContractUpdateOp::{DataContractNewDocumentTypes, DataContractNewOptionalFields},
        DocumentAction, OperationType,
    },
    Strategy,
};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};
use walkdir::WalkDir;

use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, StrategyTask, Task, StrategyCompletionResult},
    ui::screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    Event,
};

use super::{contracts_with_updates::StrategyContractsFormController, identity_inserts::StrategyIdentityInsertsFormController, start_identities::StrategyStartIdentitiesFormController, operations::StrategyAddOperationFormController, run_strategy::RunStrategyFormController, clone_strategy::CloneStrategyFormController};

const COMMAND_KEYS: [ScreenCommandKey; 11] = [
    ScreenCommandKey::new("q", "Back to Strategies"),
    ScreenCommandKey::new("r", "Run strategy"),
    ScreenCommandKey::new("l", "Clone this strategy"),
    ScreenCommandKey::new("c", "Add contract with updates"),
    ScreenCommandKey::new("w", "Remove last contract"),
    ScreenCommandKey::new("o", "Add operations"),
    ScreenCommandKey::new("z", "Remove last operation"),
    ScreenCommandKey::new("i", "Set identity inserts"),
    ScreenCommandKey::new("x", "Remove identity inserts"),
    ScreenCommandKey::new("b", "Set start identities"),
    ScreenCommandKey::new("y", "Remove start identities"),
];

const COMMAND_KEYS_NO_SELECTION: [ScreenCommandKey; 1] = [
    ScreenCommandKey::new("q", "Back to Strategies"),
];

pub struct SelectedStrategyScreenController {
    info: Info,
    available_strategies: Vec<String>,
    selected_strategy: Option<String>,
    known_contracts: BTreeMap<String, DataContract>,
    supporting_contracts: BTreeMap<String, DataContract>,
}

impl_builder!(SelectedStrategyScreenController);

impl SelectedStrategyScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let available_strategies_lock = app_state.available_strategies.lock().await;
        let selected_strategy_lock = app_state.selected_strategy.lock().await;
        let known_contracts_lock = app_state.known_contracts.lock().await;
        let supporting_contracts_lock = app_state.supporting_contracts.lock().await;

        let info = if let Some(name) = selected_strategy_lock.as_ref() {
            let strategy = available_strategies_lock
                .get(name.as_str())
                .expect("inconsistent data");
            let contract_names_lock = app_state.available_strategies_contract_names.lock().await;

            Info::new_fixed(&display_strategy(
                &name,
                strategy,
                contract_names_lock
                    .get(name.as_str())
                    .expect("inconsistent data"),
            ))
        } else {
            Info::new_fixed("No strategy selected. Go back.")
        };

        SelectedStrategyScreenController {
            info,
            available_strategies: available_strategies_lock.keys().cloned().collect(),
            selected_strategy: None,
            known_contracts: known_contracts_lock.clone(),
            supporting_contracts: supporting_contracts_lock.clone(),
        }
    }

    async fn update_supporting_contracts(&mut self, ) {
        let platform_version = PlatformVersion::latest();

        for entry in WalkDir::new("supporting_files/contract")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        {
            let path = entry.path();
            let contract_name = path.file_stem().unwrap().to_str().unwrap().to_string();

            // Change here: Add to supporting_contracts instead of known_contracts
            if !self.supporting_contracts.contains_key(&contract_name) {
                if let Ok(contract) =
                    json_document_to_created_contract(&path, true, platform_version)
                {
                    self.supporting_contracts
                        .insert(contract_name, contract.data_contract_owned());
                }
            }
        }
    }

    fn update_supporting_contracts_sync(&mut self) {
        // Use block_in_place to wait for the async operation to complete
        tokio::task::block_in_place(|| {
            // Create a new Tokio runtime for the async operation
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                self.update_supporting_contracts().await;
            })
        });
    }
}

impl ScreenController for SelectedStrategyScreenController {
    fn name(&self) -> &'static str {
        "Strategy"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        if self.selected_strategy.is_some() {
            COMMAND_KEYS.as_ref()
        } else {
            COMMAND_KEYS_NO_SELECTION.as_ref()
        }
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if self.selected_strategy.is_some() {
                    // Update known contracts before showing the form
                    self.update_supporting_contracts_sync();

                    ScreenFeedback::Form(Box::new(StrategyContractsFormController::new(
                        self.selected_strategy.clone().unwrap(),
                        self.known_contracts.clone(),
                        self.supporting_contracts.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let Some(strategy_name) = &self.selected_strategy {
                    ScreenFeedback::Form(Box::new(StrategyIdentityInsertsFormController::new(
                        strategy_name.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('b'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let Some(strategy_name) = &self.selected_strategy {
                    ScreenFeedback::Form(Box::new(StrategyStartIdentitiesFormController::new(
                        strategy_name.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('o'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let Some(strategy_name) = self.selected_strategy.clone() {
                    // Update known contracts before showing the form
                    self.update_supporting_contracts_sync();

                    ScreenFeedback::Form(Box::new(StrategyAddOperationFormController::new(
                        strategy_name.clone(),
                        self.known_contracts.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(RunStrategyFormController::new(
                self.selected_strategy.clone().unwrap(),
            ))),
            Event::Key(KeyEvent {
                code: Key::Char('w'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Strategy(StrategyTask::RemoveLastContract(
                    self.selected_strategy.clone().unwrap(),
                )),
                block: false,
            },
            Event::Key(KeyEvent {
                code: Key::Char('x'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Strategy(StrategyTask::RemoveIdentityInserts(
                    self.selected_strategy.clone().unwrap(),
                )),
                block: false,
            },
            Event::Key(KeyEvent {
                code: Key::Char('y'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Strategy(StrategyTask::RemoveStartIdentities(
                    self.selected_strategy.clone().unwrap(),
                )),
                block: false,
            },
            Event::Key(KeyEvent {
                code: Key::Char('z'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Strategy(StrategyTask::RemoveLastOperation(
                    self.selected_strategy.clone().unwrap(),
                )),
                block: false,
            },
            Event::Key(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(CloneStrategyFormController::new())),
            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name,
                    strategy,
                    contract_names,
                ))
                | BackendEvent::TaskCompletedStateChange {
                    app_state_update:
                        AppStateUpdate::SelectedStrategy(strategy_name, strategy, contract_names),
                    ..
                },
            ) => {
                self.info = Info::new_fixed(&display_strategy(
                    &strategy_name,
                    &strategy,
                    &contract_names,
                ));
                self.selected_strategy = Some(strategy_name.clone());
                ScreenFeedback::Redraw
            }
            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(strategies, ..))
                | BackendEvent::TaskCompletedStateChange {
                    app_state_update: AppStateUpdate::Strategies(strategies, ..),
                    ..
                },
            ) => {
                self.available_strategies = strategies.keys().cloned().collect();
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::StrategyCompleted {
                strategy_name,
                result,
            }) => {
                let display_text = match result {
                    StrategyCompletionResult::Success {
                        final_block_height,
                        success_count,
                        transition_count,
                        prep_time,
                        run_time,
                    } => {
                        format!(
                            "Strategy '{}' completed:\nState transitions attempted: {}\nState transitions succeeded: {}\nRun time: {:?}",
                            strategy_name,
                            transition_count,
                            success_count,
                            run_time
                        )
                    }
                    StrategyCompletionResult::PartiallyCompleted {
                        reached_block_height,
                        reason
                    } => {
                        // Handle failure case
                        format!("Strategy '{}' failed to complete. Reached block height {}. Reason: {}", strategy_name, reached_block_height, reason)
                    }
                };
    
                self.info = Info::new_fixed(&display_text);
                ScreenFeedback::Redraw
            }    
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}

fn display_strategy(
    strategy_name: &str,
    strategy: &Strategy,
    contract_updates: &[(String, Option<BTreeMap<u64, String>>)],
) -> String {
    let mut contracts_with_updates_lines = String::new();
    for (contract, updates) in contract_updates.iter() {
        contracts_with_updates_lines.push_str(&format!(
            "{:indent$}Contract: {contract}\n",
            "",
            indent = 8
        ));
        for (block, update) in updates.iter().flatten() {
            let block = block + 1;
            let block_spacing = (block - 1) * 3;
            contracts_with_updates_lines.push_str(&format!(
                "{:indent$}On block {block_spacing} apply {update}\n",
                "",
                indent = 12
            ));
        }
    }

    let times_per_block_display = if strategy.identities_inserts.times_per_block_range.end > strategy.identities_inserts.times_per_block_range.start {
        strategy.identities_inserts.times_per_block_range.end - 1
    } else {
        strategy.identities_inserts.times_per_block_range.end
    };
    
    let identity_inserts_line = format!(
        "{:indent$}Times per block: {}; chance per block: {}\n",
        "",
        times_per_block_display,
        strategy.identities_inserts.chance_per_block.unwrap_or(0.0),
        indent = 8,
    );
    
    let mut operations_lines = String::new();
    for op in strategy.operations.iter() {
        let op_name = match op.op_type.clone() {
            OperationType::Document(op) => {
                let op_type = match op.action {
                    DocumentAction::DocumentActionInsertRandom(..) => "InsertRandom".to_string(),
                    DocumentAction::DocumentActionDelete => "Delete".to_string(),
                    DocumentAction::DocumentActionReplace => "Replace".to_string(),
                    _ => panic!("invalid document action selected"),
                };
                format!("Document({})", op_type)
            }
            OperationType::IdentityTopUp => "IdentityTopUp".to_string(),
            OperationType::IdentityUpdate(op) => format!("IdentityUpdate({:?})", op),
            OperationType::IdentityWithdrawal => "IdentityWithdrawal".to_string(),
            OperationType::ContractCreate(..) => "ContractCreateRandom".to_string(),
            OperationType::ContractUpdate(op) => {
                let op_type = match op {
                    DataContractNewDocumentTypes(_) => "NewDocTypesRandom".to_string(),
                    DataContractNewOptionalFields(..) => "NewFieldsRandom".to_string(),
                };
                format!("ContractUpdate({})", op_type)
            }
            OperationType::IdentityTransfer => "IdentityTransfer".to_string(),
        };

        let times_per_block_display = if op.frequency.times_per_block_range.end > op.frequency.times_per_block_range.start {
            op.frequency.times_per_block_range.end - 1
        } else {
            op.frequency.times_per_block_range.end
        };
    
        operations_lines.push_str(&format!(
            "{:indent$}{}; Times per block: {}, chance per block: {}\n",
            "",
            op_name,
            times_per_block_display,
            op.frequency.chance_per_block.unwrap_or(0.0),
            indent = 8
        ));
    }

    format!(
        r#"{strategy_name}:
    Contracts with updates:
{contracts_with_updates_lines}
    Identity inserts:
{identity_inserts_line}
    Operations:
{operations_lines}
    Start identities: {}"#,
        strategy.start_identities.len()
    )
}
