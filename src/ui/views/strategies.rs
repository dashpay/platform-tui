//! Screens and forms related to strategies manipulation.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::Task,
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, SelectInput},
        screen::{ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey},
        views::main::MainScreenController,
    },
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Select a strategy"),
];

pub(crate) struct StrategiesScreenController;

impl ScreenController for StrategiesScreenController {
    fn name(&self) -> &'static str {
        "Strategies"
    }

    fn init_text(&self) -> &'static str {
        "Strategies management commands"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, key_event: KeyEvent) -> ScreenFeedback {
        match key_event {
            KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            } => ScreenFeedback::PreviousScreen(Box::new(MainScreenController)),
            KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            } => ScreenFeedback::Form(Box::new(SelectStrategyFormController::new(todo!()))),
            _ => ScreenFeedback::None,
        }
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
            InputStatus::Done(strategy_name) => FormStatus::Done(Task::RenderData(strategy_name)),
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
        1
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
