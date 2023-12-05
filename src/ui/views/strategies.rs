//! Screens and forms related to strategies manipulation.

mod clone_strategy;
mod contracts_with_updates;
mod delete_strategy;
mod identity_inserts;
mod new_strategy;
mod operations;
mod select_strategy;
mod start_identities;
mod run_strategy;

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

use self::{
    clone_strategy::CloneStrategyFormController,
    contracts_with_updates::StrategyContractsFormController,
    delete_strategy::DeleteStrategyFormController,
    identity_inserts::StrategyIdentityInsertsFormController,
    new_strategy::NewStrategyFormController, operations::StrategyAddOperationFormController,
    select_strategy::SelectStrategyFormController,
    start_identities::StrategyStartIdentitiesFormController,
    run_strategy::RunStrategyFormController,
};
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, StrategyTask, Task},
    ui::{
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("n", "New strategy"),
    ScreenCommandKey::new("s", "Select a strategy"),
];

const COMMANDS_KEYS_ON_STRATEGY_SELECTED: [ScreenCommandKey; 14] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("n", "New strategy"),
    ScreenCommandKey::new("s", "Select a strategy"),
    ScreenCommandKey::new("d", "Delete a strategy"),
    ScreenCommandKey::new("l", "Clone current strategy"),
    ScreenCommandKey::new("c", "Add contract with updates"),
    ScreenCommandKey::new("o", "Add operations"),
    ScreenCommandKey::new("i", "Set identity inserts"),
    ScreenCommandKey::new("b", "Set start identities"),
    ScreenCommandKey::new("r", "Run strategy"),
    ScreenCommandKey::new("w", "Remove last contract"),
    ScreenCommandKey::new("x", "Remove identity inserts"),
    ScreenCommandKey::new("y", "Remove start identities"),
    ScreenCommandKey::new("z", "Remove last operation"),
];

pub(crate) struct StrategiesScreenController {
    info: Info,
    available_strategies: Vec<String>,
    selected_strategy: Option<String>,
    known_contracts: BTreeMap<String, DataContract>,
}

impl_builder!(StrategiesScreenController);

impl StrategiesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let available_strategies_lock = app_state.available_strategies.lock().await;
        let selected_strategy_lock = app_state.selected_strategy.lock().await;
        let known_contracts_lock = app_state.known_contracts.lock().await;

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
            Info::new_fixed("Strategies management commands.\nNo selected strategy.")
        };

        StrategiesScreenController {
            info,
            available_strategies: available_strategies_lock.keys().cloned().collect(),
            selected_strategy: selected_strategy_lock.clone(),
            known_contracts: known_contracts_lock.clone(), // Clone the underlying BTreeMap
        }
    }

    async fn update_known_contracts(&mut self) {
        let platform_version = PlatformVersion::latest();

        for entry in WalkDir::new("supporting_files/contract")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        {
            let path = entry.path();
            let contract_name = path.file_stem().unwrap().to_str().unwrap().to_string();

            if !self.known_contracts.contains_key(&contract_name) {
                if let Ok(contract) =
                    json_document_to_created_contract(&path, true, platform_version)
                {
                    self.known_contracts
                        .insert(contract_name, contract.data_contract_owned());
                }
            }
        }
    }

    fn update_known_contracts_sync(&mut self) {
        // Use block_in_place to wait for the async operation to complete
        tokio::task::block_in_place(|| {
            // Create a new Tokio runtime for the async operation
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                self.update_known_contracts().await;
            })
        });
    }
}

impl ScreenController for StrategiesScreenController {
    fn name(&self) -> &'static str {
        "Strategies"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        if self.selected_strategy.is_some() {
            COMMANDS_KEYS_ON_STRATEGY_SELECTED.as_ref()
        } else {
            COMMAND_KEYS.as_ref()
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
                code: Key::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(NewStrategyFormController::new())),
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(SelectStrategyFormController::new(
                self.available_strategies.clone(),
            ))),
            Event::Key(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(DeleteStrategyFormController::new(
                self.available_strategies.clone(),
            ))),
            Event::Key(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(CloneStrategyFormController::new())),
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if self.selected_strategy.is_some() {
                    // Update known contracts before showing the form
                    self.update_known_contracts_sync();

                    ScreenFeedback::Form(Box::new(StrategyContractsFormController::new(
                        self.selected_strategy.clone().unwrap(),
                        self.known_contracts.clone(),
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
                if let Some(strategy_name) = &self.selected_strategy {
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
            contracts_with_updates_lines.push_str(&format!(
                "{:indent$}On block {block} apply {update}\n",
                "",
                indent = 12
            ));
        }
    }

    let identity_inserts_line = format!(
        "{:indent$}Times per block: {}; chance per block: {}\n",
        "",
        strategy.identities_inserts.times_per_block_range.end,
        strategy.identities_inserts.chance_per_block.unwrap_or(1.0),
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

        operations_lines.push_str(&format!(
            "{:indent$}{}; Times per block: {}, chance per block: {}\n",
            "",
            op_name,
            op.frequency.times_per_block_range.end,
            op.frequency.chance_per_block.unwrap_or(1.0),
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
