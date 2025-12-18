//! Address transfer form for strategy.
//!
//! Allows configuring address-to-address transfers in strategy tests.

use strategy_tests::{
    frequency::Frequency,
    operations::{Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus, Input,
        InputStatus, SelectInput, TextInput,
    },
};

pub(super) struct StrategyOpAddressTransferFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<u64>>>,   // Min amount (in credits)
        Field<TextInput<DefaultTextInputParser<u64>>>,   // Max amount (in credits)
        Field<TextInput<DefaultTextInputParser<u8>>>,    // Min output count
        Field<TextInput<DefaultTextInputParser<u8>>>,    // Max output count
        Field<SelectInput<f64>>,                         // Use existing addresses chance
        Field<TextInput<DefaultTextInputParser<u16>>>,   // Times per block
        Field<SelectInput<f64>>,                         // Chance per block
    )>,
    selected_strategy: String,
}

impl StrategyOpAddressTransferFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyOpAddressTransferFormController {
            input: ComposedInput::new((
                Field::new(
                    "Min amount (credits)",
                    TextInput::new("e.g. 100000000 for 0.001 DASH"),
                ),
                Field::new(
                    "Max amount (credits)",
                    TextInput::new("e.g. 1000000000 for 0.01 DASH"),
                ),
                Field::new("Min outputs", TextInput::new("e.g. 1")),
                Field::new("Max outputs", TextInput::new("e.g. 3")),
                Field::new(
                    "Use existing addresses chance",
                    SelectInput::new(vec![0.0, 0.25, 0.5, 0.75, 1.0]),
                ),
                Field::new("Times per block", TextInput::new("Enter a whole number")),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1, 0.05, 0.01]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyOpAddressTransferFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((
                min_amount,
                max_amount,
                min_outputs,
                max_outputs,
                use_existing_chance,
                times_per_block,
                chance_per_block,
            )) => {
                // Convert use_existing_chance to Option (0.0 means None/disabled)
                let use_existing = if use_existing_chance > 0.0 {
                    Some(use_existing_chance)
                } else {
                    None
                };

                FormStatus::Done {
                    task: Task::Strategy(StrategyTask::AddOperation {
                        strategy_name: self.selected_strategy.clone(),
                        operation: Operation {
                            op_type: OperationType::AddressTransfer(
                                min_amount..=max_amount,
                                min_outputs..=max_outputs,
                                use_existing,
                                None, // Fee strategy - use default
                            ),
                            frequency: Frequency {
                                times_per_block_range: times_per_block..times_per_block + 1,
                                chance_per_block: Some(chance_per_block),
                            },
                        },
                    }),
                    block: false,
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Address transfer operation"
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
        7
    }
}
