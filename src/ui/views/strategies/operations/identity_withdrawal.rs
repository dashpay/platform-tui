//! Identity withdrawal operations form for strategy.

use strategy_tests::{
    frequency::Frequency,
    operations::{Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpIdentityWithdrawalFormController {
    input: SelectInput<f64>,
    selected_strategy: String,
}

impl StrategyOpIdentityWithdrawalFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyOpIdentityWithdrawalFormController {
            input: SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1]),
            selected_strategy,
        }
    }
}

impl FormController for StrategyOpIdentityWithdrawalFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(chance_per_block) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::AddOperation {
                    strategy_name: self.selected_strategy.clone(),
                    operation: Operation {
                        op_type: OperationType::IdentityWithdrawal(10_000_000..=15_000_000),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: Some(chance_per_block),
                        },
                    },
                }),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity withdrawal operation"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Chance per block"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
