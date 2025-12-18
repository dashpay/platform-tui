//! Start addresses screen and forms.

use dpp::fee::Credits;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus, Input,
        InputStatus, TextInput,
    },
    ui::screen::{
        utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

use strategy_tests::Strategy;

const COMMAND_KEYS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Back to Strategy"),
    ScreenCommandKey::new("a", "Add/edit"),
    ScreenCommandKey::new("c", "Remove"),
];

pub(crate) struct StartAddressesScreenController {
    info: Info,
    strategy_name: Option<String>,
    selected_strategy: Option<Strategy>,
}

impl_builder!(StartAddressesScreenController);

impl StartAddressesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let available_strategies_lock = app_state.available_strategies.lock().await;
        let selected_strategy_lock = app_state.selected_strategy.lock().await;

        let (info_text, current_strategy) =
            if let Some(selected_strategy_name) = &*selected_strategy_lock {
                if let Some(strategy) = available_strategies_lock.get(selected_strategy_name) {
                    let info_text = format!("Selected Strategy: {}", selected_strategy_name);
                    (info_text, Some(strategy.clone()))
                } else {
                    ("No selected strategy found".to_string(), None)
                }
            } else {
                ("No strategy selected".to_string(), None)
            };

        let info = Info::new_fixed(&info_text);

        Self {
            info,
            strategy_name: selected_strategy_lock.clone(),
            selected_strategy: current_strategy,
        }
    }
}

impl ScreenController for StartAddressesScreenController {
    fn name(&self) -> &'static str {
        "Start addresses"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
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
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let Some(strategy_name) = &self.strategy_name {
                    ScreenFeedback::Form(Box::new(StrategyStartAddressesFormController::new(
                        strategy_name.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if let Some(strategy_name) = &self.strategy_name {
                    ScreenFeedback::Task {
                        task: Task::Strategy(StrategyTask::RemoveStartAddresses(
                            strategy_name.clone(),
                        )),
                        block: false,
                    }
                } else {
                    ScreenFeedback::None
                }
            }

            // Backend events
            Event::Backend(BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                strategy_name,
                strategy,
                _,
            ))) => {
                if Some(strategy_name) == self.strategy_name.as_ref() {
                    self.selected_strategy = Some((*strategy).clone());
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Strategy(StrategyTask::SetStartAddresses { .. }),
                app_state_update:
                    AppStateUpdate::SelectedStrategy(strategy_name, updated_strategy, _),
                ..
            }) => {
                if Some(&strategy_name) == self.strategy_name.as_ref().as_ref() {
                    self.selected_strategy = Some((*updated_strategy).clone());
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Strategy(StrategyTask::RemoveStartAddresses(_)),
                app_state_update:
                    AppStateUpdate::SelectedStrategy(strategy_name, updated_strategy, _),
                ..
            }) => {
                if Some(&strategy_name) == self.strategy_name.as_ref().as_ref() {
                    self.selected_strategy = Some((*updated_strategy).clone());
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }

            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let display_text = if let Some(strategy) = &self.selected_strategy {
            let start_addresses_text = format!(
                "Number of addresses: {}\nStarting balance: {:.4} dash ({} credits)",
                strategy.start_addresses.number_of_addresses,
                strategy.start_addresses.starting_balance as f64 / 100_000_000_000.0,
                strategy.start_addresses.starting_balance,
            );

            format!(
                "Strategy: {}\n\n{}",
                self.strategy_name
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string()),
                start_addresses_text,
            )
        } else {
            "Select a strategy to view start addresses.".to_string()
        };

        self.info = Info::new_fixed(&display_text);
        self.info.view(frame, area);
    }
}

pub(super) struct StrategyStartAddressesFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<u16>>>,
        Field<TextInput<DefaultTextInputParser<f64>>>,
    )>,
    selected_strategy: String,
}

impl StrategyStartAddressesFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyStartAddressesFormController {
            input: ComposedInput::new((
                Field::new(
                    "Number of addresses",
                    TextInput::new("Enter a whole number"),
                ),
                Field::new(
                    "Starting balance (in Dash)",
                    TextInput::new("e.g. 0.01 for 0.01 DASH"),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyStartAddressesFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((count, balance_dash)) => {
                let balance_credits: Credits = (balance_dash * 100_000_000_000.0) as Credits;
                FormStatus::Done {
                    task: Task::Strategy(StrategyTask::SetStartAddresses {
                        strategy_name: self.selected_strategy.clone(),
                        count,
                        balance: balance_credits,
                    }),
                    block: false,
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Start addresses for strategy"
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
