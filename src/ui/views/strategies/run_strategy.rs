//! Run strategy screen and forms.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, BackendEvent, StrategyCompletionResult, StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus, Input,
        InputStatus, SelectInput, TextInput,
    },
    ui::screen::{
        utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Strategy"),
    ScreenCommandKey::new("r", "Rerun strategy"),
];

pub(crate) struct RunStrategyScreenController {
    info: Info,
    strategy_running: bool,
    selected_strategy: Option<String>,
}

impl_builder!(RunStrategyScreenController);

impl RunStrategyScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let selected_strategy_lock = app_state.selected_strategy.lock().await;

        let (info, strategy_running, selected_strategy) =
            if let Some(current_strategy) = selected_strategy_lock.as_ref() {
                let info = Info::new_fixed("Strategy is running, please wait.");
                (info, true, Some(current_strategy.clone()))
            } else {
                let info = Info::new_error("Run strategy not confirmed.");
                (info, false, None)
            };

        drop(selected_strategy_lock);

        Self {
            info,
            strategy_running,
            selected_strategy,
        }
    }
}

impl ScreenController for RunStrategyScreenController {
    fn name(&self) -> &'static str {
        "Run strategy"
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
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.strategy_running = true;
                ScreenFeedback::Form(Box::new(RunStrategyFormController::new(
                    self.selected_strategy
                        .clone()
                        .expect("No selected strategy available"),
                )))
            }
            Event::Backend(BackendEvent::StrategyCompleted {
                strategy_name,
                result,
            }) => {
                self.strategy_running = false;

                let display_text = match result {
                    StrategyCompletionResult::Success {
                        block_mode,
                        final_block_height,
                        start_block_height,
                        success_count,
                        transition_count,
                        rate,
                        success_rate,
                        success_percent,
                        run_time,
                        init_time,
                        dash_spent_identity,
                        dash_spent_wallet,
                    } => {
                        let mode = match block_mode {
                            true => String::from("block"),
                            false => String::from("time"),
                        };
                        format!(
                            "Strategy '{}' completed:\n\nMode: {}\nState transitions attempted: {}\nState \
                             transitions succeeded: {}\nNumber of blocks (or loops): {}\nRun time: \
                             {}s\nInitialization time: {}\nAttempted rate (approx): {:.2} tx/s\nSuccessful rate: {:.2} tx/s\nSuccess percentage: {}%\nDash spent (Identity): {}\nDash spent (Wallet): {}",
                            strategy_name,
                            mode,
                            transition_count,
                            success_count,
                            (final_block_height - start_block_height - 2), // -2 so we don't count the 2 init blocks
                            run_time.as_secs(),
                            init_time.as_secs(),
                            rate,
                            success_rate,
                            success_percent,
                            dash_spent_identity,
                            dash_spent_wallet,
                        )
                    }
                    StrategyCompletionResult::PartiallyCompleted {
                        reached_block_height,
                        reason,
                    } => {
                        format!(
                            "Strategy '{}' failed to complete. Reached block/second {}. Reason: {}",
                            strategy_name, reached_block_height, reason
                        )
                    }
                };

                self.info = Info::new_fixed(&display_text);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::StrategyError { error }) => {
                self.strategy_running = false;

                self.info = Info::new_error(&format!("Strategy error: {}", &error));
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if self.strategy_running {
            self.info = Info::new_fixed("Strategy is running, please wait.");
        }
        self.info.view(frame, area)
    }
}

pub(super) struct RunStrategyFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<u64>>>, // Number of blocks or seconds
        Field<TextInput<DefaultTextInputParser<u64>>>, // Seconds per loop
        Field<SelectInput<String>>,                    // Verify proofs?
        // Field<TextInput<DefaultTextInputParser<f64>>>, // Top up amount
        Field<SelectInput<String>>, // Confirm
    )>,
    selected_strategy: String,
}

impl RunStrategyFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        RunStrategyFormController {
            input: ComposedInput::new((
                Field::new(
                    "Number of seconds to run the strategy",
                    TextInput::new("Enter a whole number"),
                ),
                Field::new(
                    "Number of seconds between each batch of transitions",
                    TextInput::new("Enter a whole number"),
                ),
                Field::new(
                    "Verify proofs?",
                    SelectInput::new(vec!["No".to_string(), "Yes".to_string()]),
                ),
                // Field::new(
                //     "Amount to top up identities. Enter 0 for none.",
                //     TextInput::new("Enter Dash amount (decimals ok)."),
                // ),
                Field::new(
                    "Confirm start",
                    SelectInput::new(vec!["No".to_string(), "Yes".to_string()]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for RunStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((
                num_blocks,
                seconds_per_loop,
                verify_proofs,
                // top_up_amount_dash,
                confirm,
            )) => {
                // let top_up_amount_credits = (top_up_amount_dash * 100_000_000_000.0) as u64;
                if confirm == "Yes" {
                    if verify_proofs == "Yes" {
                        FormStatus::Done {
                            task: Task::Strategy(StrategyTask::RunStrategy(
                                self.selected_strategy.clone(),
                                num_blocks,
                                seconds_per_loop,
                                true,
                                0, // top up amount
                            )),
                            block: true,
                        }
                    } else {
                        FormStatus::Done {
                            task: Task::Strategy(StrategyTask::RunStrategy(
                                self.selected_strategy.clone(),
                                num_blocks,
                                seconds_per_loop,
                                false,
                                0, // top up amount
                            )),
                            block: true,
                        }
                    }
                } else {
                    FormStatus::PreviousScreen
                }
            }
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Run strategy"
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
        self.input.steps_number()
    }
}
