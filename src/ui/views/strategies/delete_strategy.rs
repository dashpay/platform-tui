//! Strategy deletion form.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct DeleteStrategyFormController {
    input: SelectInput<String>,
}

impl DeleteStrategyFormController {
    pub(super) fn new(strategies: Vec<String>) -> Self {
        DeleteStrategyFormController {
            input: SelectInput::new(strategies),
        }
    }
}

impl FormController for DeleteStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(strategy_name) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::DeleteStrategy(strategy_name)),
                block: false,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Strategy deletion"
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
