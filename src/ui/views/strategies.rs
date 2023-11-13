//! Screens and forms related to strategies manipulation.

mod identity_inserts;
mod operations;
mod select_strategy;
mod start_identities;

use std::collections::BTreeMap;

use strategy_tests::Strategy;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use self::{
    identity_inserts::StrategyIdentityInsertsFormController,
    operations::StrategyAddOperationFormController, select_strategy::SelectStrategyFormController,
    start_identities::StrategyStartIdentitiesFormController,
};
use crate::{
    backend::{AppState, BackendEvent},
    ui::{
        screen::{
            widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback,
            ScreenToggleKey,
        },
        views::main::MainScreenController,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Select a strategy"),
];

const COMMANDS_KEYS_ON_STRATEGY_SELECTED: [ScreenCommandKey; 9] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Select a strategy"),
    ScreenCommandKey::new("c", "Set contracts with updates"),
    ScreenCommandKey::new("i", "Set identity inserts"),
    ScreenCommandKey::new("b", "Set start identities"),
    ScreenCommandKey::new("o", "Add operations"),
    ScreenCommandKey::new("Ctrl-i", "Remove identity inserts"),
    ScreenCommandKey::new("Ctrl-b", "Remove start identities"),
    ScreenCommandKey::new("Ctrl-o", "Remove operations"),
];

pub(crate) struct StrategiesScreenController {
    info: Info,
    available_strategies: Vec<String>,
    selected_strategy: Option<String>,
}

impl StrategiesScreenController {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let info = Self::build_info(app_state);

        StrategiesScreenController {
            info,
            available_strategies: app_state.available_strategies.keys().cloned().collect(),
            selected_strategy: app_state.selected_strategy.clone(),
        }
    }

    fn build_info(app_state: &AppState) -> Info {
        let selected_strategy_name = app_state.selected_strategy.as_ref();
        let selected_strategy = selected_strategy_name
            .map(|s| app_state.available_strategies.get(s.as_str()))
            .flatten();
        let selected_strategy_contracts_updates = selected_strategy_name
            .map(|s| {
                app_state
                    .available_strategies_contract_names
                    .get(s.as_str())
            })
            .flatten();

        if let (Some(name), Some(strategy), Some(contract_updates)) = (
            selected_strategy_name,
            selected_strategy,
            selected_strategy_contracts_updates,
        ) {
            Info::new_fixed(&display_strategy(name, strategy, contract_updates))
        } else {
            Info::new_fixed("Strategies management commands.\nNo selected strategy.")
        }
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

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => {
                ScreenFeedback::PreviousScreen(Box::new(|_| Box::new(MainScreenController::new())))
            }
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(SelectStrategyFormController::new(
                self.available_strategies.clone(),
            ))),
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
                    )))
                } else {
                    ScreenFeedback::None
                }
            }

            Event::Backend(
                BackendEvent::AppStateUpdated(app_state)
                | BackendEvent::TaskCompletedStateChange(_, app_state),
            ) => {
                self.info = Self::build_info(&app_state);
                if let Some(strategy_name) = &app_state.selected_strategy {
                    self.selected_strategy = Some(strategy_name.clone());
                }
                ScreenFeedback::Redraw
            }

            // Event::Backend(BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy {})) =>
            // {}
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
        operations_lines.push_str(&format!(
            "{:indent$}{:?}; Times per block: {}, chance per block: {}\n",
            "",
            op.op_type,
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
