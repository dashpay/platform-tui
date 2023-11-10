//! Screens and forms related to strategies manipulation.

use strategy_tests::Strategy;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{AppState, Task},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, SelectInput},
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

pub(crate) struct StrategiesScreenController {
    info: Info,
    available_strategies: Vec<String>,
}

impl StrategiesScreenController {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let selected_strategy = app_state
            .selected_strategy
            .as_ref()
            .map(|s| app_state.available_strategies.get(s.as_str()))
            .flatten();

        StrategiesScreenController {
            info: Info::new_fixed("Strategies management commands"),
            available_strategies: app_state.available_strategies.keys().cloned().collect(),
        }
    }
}

impl ScreenController for StrategiesScreenController {
    fn name(&self) -> &'static str {
        "Strategies"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
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

fn display_strategy(strategy: &Strategy) -> String {
    todo!()
}
