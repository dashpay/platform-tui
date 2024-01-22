//! Run strategy confirmation.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct RunStrategyFormController {
    input: SelectInput<String>,
    selected_strategy: String,
}

impl RunStrategyFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        RunStrategyFormController {
            input: SelectInput::new(vec!["Yes".to_string(), "No".to_string()]),
            selected_strategy,
        }
    }
}

impl FormController for RunStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(selection) => {
                match selection.as_str() {
                    "Yes" => FormStatus::Done {
                        task: Task::Strategy(StrategyTask::RunStrategy(self.selected_strategy.clone())),
                        block: true,
                    },
                    "No" => FormStatus::Exit,
                    _ => FormStatus::Exit,
                }
            },
            status => status.into(),
        }
    }
    
    fn form_name(&self) -> &'static str {
        "Confirm you would like to run the strategy."
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Run strategy?"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
