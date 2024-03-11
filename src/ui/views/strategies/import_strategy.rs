//! Form to import a Strategy from Github via URL.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus, TextInput,
    },
};

pub(crate) struct ImportStrategyFormController {
    input: TextInput<DefaultTextInputParser<String>>,
}

impl ImportStrategyFormController {
    pub(crate) fn new() -> Self {
        Self {
            input: TextInput::new("Github file URL"),
        }
    }
}

impl FormController for ImportStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(url) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::ImportStrategy(url)),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Import strategy"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Url"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
