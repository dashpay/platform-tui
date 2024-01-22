//! Strategy deletion form.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct DeleteStrategyFormController {
    strategy_input: SelectInput<String>,
    confirm_input: SelectInput<String>,
    selected_strategy: Option<String>,
    step: u8,
}

impl DeleteStrategyFormController {
    pub(super) fn new(strategies: Vec<String>) -> Self {
        DeleteStrategyFormController {
            strategy_input: SelectInput::new(strategies),
            confirm_input: SelectInput::new(vec!["Yes".to_string(), "No".to_string()]),
            selected_strategy: None,
            step: 0,
        }
    }
}

impl FormController for DeleteStrategyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.step {
            0 => match self.strategy_input.on_event(event) {
                InputStatus::Done(strategy_name) => {
                    self.selected_strategy = Some(strategy_name);
                    self.step = 1;
                    FormStatus::Redraw
                },
                status => status.into(),
            },
            1 => match self.confirm_input.on_event(event) {
                InputStatus::Done(choice) => {
                    if choice == "Yes" {
                        FormStatus::Done {
                            task: Task::Strategy(StrategyTask::DeleteStrategy(self.selected_strategy.clone().unwrap())),
                            block: false,
                        }
                    } else {
                        FormStatus::Exit
                    }
                },
                status => status.into(),
            },
            _ => FormStatus::None,
        }
    }

    fn form_name(&self) -> &'static str {
        "Strategy deletion"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        match self.step {
            0 => self.strategy_input.view(frame, area),
            1 => self.confirm_input.view(frame, area),
            _ => {}
        }
    }

    fn step_name(&self) -> &'static str {
        match self.step {
            0 => "Select Strategy",
            1 => "Confirm Deletion",
            _ => "",
        }
    }

    fn step_index(&self) -> u8 {
        self.step
    }

    fn steps_number(&self) -> u8 {
        2
    }
}
