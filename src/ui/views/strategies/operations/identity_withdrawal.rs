//! Identity withdrawal operations form for strategy.

use strategy_tests::{
    frequency::Frequency,
    operations::{Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpIdentityWithdrawalFormController {
    input: ComposedInput<(Field<SelectInput<u16>>, Field<SelectInput<f64>>)>,
    selected_strategy: String,
}

impl StrategyOpIdentityWithdrawalFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyOpIdentityWithdrawalFormController {
            input: ComposedInput::new((
                Field::new(
                    "Times per block",
                    SelectInput::new(vec![1, 2, 5, 10, 20, 40, 100, 1000]),
                ),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1, 0.05, 0.01]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyOpIdentityWithdrawalFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((times_per_block, chance_per_block)) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::AddOperation {
                    strategy_name: self.selected_strategy.clone(),
                    operation: Operation {
                        op_type: OperationType::IdentityWithdrawal,
                        frequency: Frequency {
                            times_per_block_range: 1..times_per_block + 1,
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
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        2
    }
}
