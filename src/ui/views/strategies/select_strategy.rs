//! Strategy selection form.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct SelectStrategyFormController {
    input: SelectInput<String>,
}

impl SelectStrategyFormController {
    pub(super) fn new(strategies: Vec<String>) -> Self {
        SelectStrategyFormController {
            input: SelectInput::new(strategies),
        }
    }
}

impl FormController for SelectStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(strategy_name) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::SelectStrategy(strategy_name)),
                block: false,
            },
            InputStatus::Exit => FormStatus::Exit,
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Strategy selection"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        ""
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
