//! Forms for strategy operations related to identity updates.

use strategy_tests::{
    frequency::Frequency,
    operations::{IdentityUpdateOp, Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpIdentityUpdateFormController {
    input: ComposedInput<(
        Field<SelectInput<u16>>,
        // Field<SelectInput<u16>>,
        Field<SelectInput<f64>>,
    )>,
    selected_strategy: String,
    key_update_op: KeyUpdateOp,
}

pub(super) enum KeyUpdateOp {
    AddKeys,
    DisableKeys,
}

impl StrategyOpIdentityUpdateFormController {
    pub(super) fn new(selected_strategy: String, key_update_op: KeyUpdateOp) -> Self {
        let count_message = match key_update_op {
            KeyUpdateOp::AddKeys => "How many keys to add",
            KeyUpdateOp::DisableKeys => "How many keys to disable",
        };
        StrategyOpIdentityUpdateFormController {
            input: ComposedInput::new((
                Field::new(
                    count_message,
                    SelectInput::new(vec![1, 2, 5, 10, 20, 40, 100, 1000]),
                ),
                // Field::new(
                //     "Times per block",
                //     SelectInput::new(vec![1, 2, 5, 10, 20, 40, 100, 1000]),
                // ),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1]),
                ),
            )),
            selected_strategy,
            key_update_op,
        }
    }
}

impl FormController for StrategyOpIdentityUpdateFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((count, chance_per_block)) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::AddOperation {
                    strategy_name: self.selected_strategy.clone(),
                    operation: Operation {
                        op_type: OperationType::IdentityUpdate(match self.key_update_op {
                            KeyUpdateOp::AddKeys => IdentityUpdateOp::IdentityUpdateAddKeys(count),
                            KeyUpdateOp::DisableKeys => {
                                IdentityUpdateOp::IdentityUpdateDisableKey(count)
                            }
                        }),
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
        "Identity keys updates operation"
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
        3
    }
}
