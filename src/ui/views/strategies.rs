//! Screens and forms related to strategies manipulation.

use std::collections::BTreeMap;

use strategy_tests::{frequency::Frequency, Strategy};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, BackendEvent, Task},
    ui::{
        form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
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

// TODO: maybe write a macro to reduce duplication
const COMMANDS_KEYS_ON_STRATEGY_SELECTED: [ScreenCommandKey; 5] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Select a strategy"),
    ScreenCommandKey::new("c", "Set contracts with updates"),
    ScreenCommandKey::new("i", "Set identity inserts"),
    ScreenCommandKey::new("b", "Set start identities"),
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

pub(crate) struct SelectStrategyFormController {
    input: SelectInput<String>,
}

impl SelectStrategyFormController {
    pub(crate) fn new(strategies: Vec<String>) -> Self {
        SelectStrategyFormController {
            input: SelectInput::new(strategies),
        }
    }
}

impl FormController for SelectStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(strategy_name) => FormStatus::Done {
                task: Task::SelectStrategy(strategy_name),
                block: false,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
        }
    }

    fn form_name(&self) -> &'static str {
        "Strategy selection"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "By name"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

struct StrategyIdentityInsertsFormController {
    input: ComposedInput<(Field<SelectInput<u16>>, Field<SelectInput<f64>>)>,
    selected_strategy: String,
}

impl StrategyIdentityInsertsFormController {
    fn new(selected_strategy: String) -> Self {
        StrategyIdentityInsertsFormController {
            input: ComposedInput::new((
                Field::new(
                    "Times per block",
                    SelectInput::new(vec![1, 2, 5, 10, 20, 40, 100, 1000]),
                ),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1, 0.05, 0.01]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyIdentityInsertsFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((count, chance)) => FormStatus::Done {
                task: Task::StrategySetIdentityInserts {
                    strategy_name: self.selected_strategy.clone(),
                    identity_inserts_frequency: Frequency {
                        times_per_block_range: 1..count,
                        chance_per_block: Some(chance),
                    },
                },
                block: false,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity inserts for strategy"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        2
    }
}

struct StrategyStartIdentitiesFormController {
    input: ComposedInput<(Field<SelectInput<u16>>, Field<SelectInput<u32>>)>,
    selected_strategy: String,
}

impl StrategyStartIdentitiesFormController {
    fn new(selected_strategy: String) -> Self {
        StrategyStartIdentitiesFormController {
            input: ComposedInput::new((
                Field::new(
                    "Number of identities",
                    SelectInput::new(vec![1, 10, 100, 1000, 10000, u16::MAX]),
                ),
                Field::new(
                    "Keys per identity",
                    SelectInput::new(vec![2, 3, 4, 5, 10, 20, 32]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyStartIdentitiesFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((count, key_count)) => FormStatus::Done {
                task: Task::StrategyStartIdentities {
                    strategy_name: self.selected_strategy.clone(),
                    count,
                    key_count,
                },
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity inserts for strategy"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        2
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
