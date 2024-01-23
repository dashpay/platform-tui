//! Screens and forms related to strategies manipulation.

mod clone_strategy;
mod contracts_with_updates;
mod contracts_with_updates_screen;
mod delete_strategy;
mod identity_inserts;
mod new_strategy;
mod operations;
mod select_strategy;
mod start_identities;
mod run_strategy;
mod run_strategy_screen;
pub mod selected_strategy;

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use self::{
    delete_strategy::DeleteStrategyFormController,
    new_strategy::NewStrategyFormController,
    select_strategy::SelectStrategyFormController, selected_strategy::SelectedStrategyScreenController,
};
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent},
    ui::screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 4] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("n", "New strategy"),
    ScreenCommandKey::new("s", "Select a strategy"),
    ScreenCommandKey::new("d", "Delete a strategy"),
];

pub(crate) struct StrategiesScreenController {
    info: Info,
    available_strategies: Vec<String>,
    selected_strategy: Option<String>,
}

impl_builder!(StrategiesScreenController);

impl StrategiesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let available_strategies_lock = app_state.available_strategies.lock().await;
        let strategies = available_strategies_lock.keys().cloned().collect::<Vec<_>>();
        let info_text = if strategies.is_empty() {
            "No available strategies".to_string()
        } else {
            let strategy_list = strategies
                .iter()
                .map(|strategy_name| format!(" - {}", strategy_name))
                .collect::<Vec<_>>()
                .join("\n");

            format!("Available strategies:\n{}", strategy_list)
        };

        let info = Info::new_fixed(&info_text);

        StrategiesScreenController {
            info,
            available_strategies: strategies,
            selected_strategy: None,
        }
    }
}

impl ScreenController for StrategiesScreenController {
    fn name(&self) -> &'static str {
        "Strategies"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        if self.available_strategies.is_empty() {
            &COMMAND_KEYS[..2] // Exclude Delete and Select when no strategies loaded
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
            }) => ScreenFeedback::FormThenNextScreen {
                form: Box::new(NewStrategyFormController::new()), 
                screen: SelectedStrategyScreenController::builder()
            },
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::FormThenNextScreen {
                form: Box::new(SelectStrategyFormController::new(self.available_strategies.clone())),
                screen: SelectedStrategyScreenController::builder()
            },
            Event::Key(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(DeleteStrategyFormController::new(
                self.available_strategies.clone(),
            ))),
            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name,
                    _strategy,
                    _contract_names,
                ))
                | BackendEvent::TaskCompletedStateChange {
                    app_state_update:
                        AppStateUpdate::SelectedStrategy(strategy_name, _strategy, _contract_names),
                    ..
                },
            ) => {
                self.selected_strategy = Some(strategy_name.clone());
                if !self.available_strategies.contains(&strategy_name) {
                    self.available_strategies.push(strategy_name.clone());
                }
                ScreenFeedback::Redraw
            }
            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(strategies, ..)),
                ..
            ) => {
                self.available_strategies = strategies.keys().cloned().collect();
            
                let info_text = if self.available_strategies.is_empty() {
                    "No available strategies".to_string()
                } else {
                    let strategy_list = self.available_strategies
                        .iter()
                        .map(|strategy_name| format!(" - {}", strategy_name))
                        .collect::<Vec<_>>()
                        .join("\n");
            
                    format!("Available strategies:\n{}", strategy_list)
                };
            
                self.info = Info::new_fixed(&info_text);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}