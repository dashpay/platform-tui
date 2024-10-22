//! Strategy's identity inserts form.

use dpp::dash_to_credits;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, StrategyTask, Task},
    ui::{
        form::{
            parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus,
            Input, InputStatus, SelectInput, TextInput,
        },
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

use strategy_tests::{frequency::Frequency, Strategy};

const COMMAND_KEYS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Back to Strategy"),
    ScreenCommandKey::new("a", "Add/edit"),
    ScreenCommandKey::new("c", "Clear"),
];

pub(crate) struct IdentityInsertsScreenController {
    info: Info,
    strategy_name: Option<String>,
    selected_strategy: Option<Strategy>,
}

impl_builder!(IdentityInsertsScreenController);

impl IdentityInsertsScreenController {
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

impl ScreenController for IdentityInsertsScreenController {
    fn name(&self) -> &'static str {
        "Identity inserts"
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
                    ScreenFeedback::Form(Box::new(StrategyIdentityInsertsFormController::new(
                        strategy_name.clone(),
                    )))
                } else {
                    ScreenFeedback::None
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Strategy(StrategyTask::RemoveIdentityInserts(
                    self.strategy_name.clone().unwrap(),
                )),
                block: false,
            },
            Event::Backend(BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                strategy_name,
                strategy,
                _,
            ))) => {
                // Check if the updated strategy is the one currently being displayed
                if Some(strategy_name) == self.strategy_name.as_ref() {
                    // Update the selected_strategy with the new data
                    self.selected_strategy = Some((*strategy).clone());

                    // Trigger a redraw of the screen
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
            // Extracting times per block and chance per block from
            // strategy.identity_inserts
            let times_per_block_display = if strategy
                .identity_inserts
                .frequency
                .times_per_block_range
                .end
                > strategy
                    .identity_inserts
                    .frequency
                    .times_per_block_range
                    .start
            {
                strategy
                    .identity_inserts
                    .frequency
                    .times_per_block_range
                    .end
                    - 1
            } else {
                strategy
                    .identity_inserts
                    .frequency
                    .times_per_block_range
                    .end
            };

            let mut identity_inserts_text = String::new();

            if times_per_block_display == 0 {
                identity_inserts_text = format!(
                    "Identity inserts:\nTimes per block: {}; Chance per block: {}",
                    times_per_block_display,
                    strategy
                        .identity_inserts
                        .frequency
                        .chance_per_block
                        .unwrap_or(0.0),
                );
            } else {
                identity_inserts_text = format!(
                    "Identity inserts:\nTimes per block: {}; Chance per block: {}",
                    times_per_block_display,
                    strategy
                        .identity_inserts
                        .frequency
                        .chance_per_block
                        .unwrap_or(0.0),
                );
            }

            format!(
                "Strategy: {}\n{}",
                self.strategy_name
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string()),
                identity_inserts_text
            )
        } else {
            "Select a strategy to view identity inserts.".to_string()
        };

        self.info = Info::new_fixed(&display_text);
        self.info.view(frame, area);
    }
}

pub(super) struct StrategyIdentityInsertsFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<u16>>>,
        Field<SelectInput<f64>>,
    )>,
    selected_strategy: String,
}

impl StrategyIdentityInsertsFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyIdentityInsertsFormController {
            input: ComposedInput::new((
                Field::new("Times per block", TextInput::new("Enter a whole number")),
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
                task: Task::Strategy(StrategyTask::SetIdentityInserts {
                    strategy_name: self.selected_strategy.clone(),
                    identity_inserts_frequency: Frequency {
                        times_per_block_range: count..count + 1,
                        chance_per_block: Some(chance),
                    },
                    start_balance: dash_to_credits!(1), //todo, set this
                }),
                block: false,
            },
            status => status.into(),
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
