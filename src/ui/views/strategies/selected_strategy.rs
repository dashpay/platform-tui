//! Screens and forms related to strategies manipulation.

use std::collections::BTreeMap;

use strategy_tests::{
    operations::{
        DataContractUpdateAction::{DataContractNewDocumentTypes, DataContractNewOptionalFields},
        DocumentAction, OperationType,
    },
    Strategy,
};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::{
    clone_strategy::CloneStrategyFormController,
    contracts_with_updates_screen::ContractsWithUpdatesScreenController,
    identity_inserts_screen::IdentityInsertsScreenController,
    operations_screen::OperationsScreenController, run_strategy::RunStrategyFormController,
    run_strategy_screen::RunStrategyScreenController,
    start_identities_screen::StartIdentitiesScreenController,
};
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent},
    ui::screen::{
        utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 7] = [
    ScreenCommandKey::new("q", "Back to Strategies"),
    ScreenCommandKey::new("r", "Run strategy"),
    ScreenCommandKey::new("l", "Clone this strategy"),
    ScreenCommandKey::new("c", "Contracts with updates"),
    ScreenCommandKey::new("i", "Identity inserts"),
    ScreenCommandKey::new("o", "Operations"),
    ScreenCommandKey::new("s", "Start identities"),
];

const COMMAND_KEYS_NO_SELECTION: [ScreenCommandKey; 1] =
    [ScreenCommandKey::new("q", "Back to Strategies")];

pub struct SelectedStrategyScreenController {
    info: Info,
    available_strategies: Vec<String>,
    selected_strategy: Option<String>,
}

impl_builder!(SelectedStrategyScreenController);

impl SelectedStrategyScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let available_strategies_lock = app_state.available_strategies.lock().await;
        let selected_strategy_lock = app_state.selected_strategy.lock().await;

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
        }
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
            }) => ScreenFeedback::NextScreen(ContractsWithUpdatesScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(IdentityInsertsScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(StartIdentitiesScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('o'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(OperationsScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::FormThenNextScreen {
                form: Box::new(RunStrategyFormController::new(
                    self.selected_strategy.clone().unwrap(),
                )),
                screen: RunStrategyScreenController::builder(),
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

    let times_per_block_display = if strategy.identities_inserts.times_per_block_range.end
        > strategy.identities_inserts.times_per_block_range.start
    {
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
                let op_type = match op.action {
                    DataContractNewDocumentTypes(_) => "NewDocTypesRandom".to_string(),
                    DataContractNewOptionalFields(..) => "NewFieldsRandom".to_string(),
                };
                format!("ContractUpdate({})", op_type)
            }
            OperationType::IdentityTransfer => "IdentityTransfer".to_string(),
        };

        let times_per_block_display =
            if op.frequency.times_per_block_range.end > op.frequency.times_per_block_range.start {
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
        strategy.start_identities.0
    )
}
