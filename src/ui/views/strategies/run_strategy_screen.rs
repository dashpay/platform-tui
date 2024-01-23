//! Run strategy screen.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, BackendEvent, StrategyCompletionResult},
    ui::screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 1] = [
    ScreenCommandKey::new("q", "Back to Strategy"),
];

pub(crate) struct RunStrategyScreenController {
    info: Info,
    strategy_running: bool,
}

impl_builder!(RunStrategyScreenController);

impl RunStrategyScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {        
        let info = Info::new_fixed("Strategy is running, please wait. \nCheck `explorer.log` file to watch progress.");

        Self {
            info,
            strategy_running: true,
        }
    }
}

impl ScreenController for RunStrategyScreenController {
    fn name(&self) -> &'static str {
        "Run Strategy"
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
            Event::Backend(BackendEvent::StrategyCompleted {
                strategy_name,
                result,
            }) => {
                self.strategy_running = false;

                let display_text = match result {
                    StrategyCompletionResult::Success {
                        final_block_height: _final_block_height,
                        success_count,
                        transition_count,
                        prep_time: _prep_time,
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
            Event::Backend(BackendEvent::StrategyError {
                strategy_name,
                error,
            }) => {
                self.strategy_running = false;

                self.info = Info::new_fixed(&format!("Error running strategy {}: {}", strategy_name, &error));
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if self.strategy_running {
            self.info = Info::new_fixed("Strategy is running, please wait. \nCheck `explorer.log` file to watch progress.");
        }
        self.info.view(frame, area)
    }
}