//! Form to clone the existing strategy with new name.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus, TextInput,
    },
};

pub(crate) struct CloneStrategyFormController {
    input: TextInput<DefaultTextInputParser<String>>,
}

impl CloneStrategyFormController {
    pub(crate) fn new() -> Self {
        CloneStrategyFormController {
            input: TextInput::new("strategy name"),
        }
    }
}

impl FormController for CloneStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(strategy_name) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::CloneStrategy(strategy_name)),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Clone strategy"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Strategy name"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
