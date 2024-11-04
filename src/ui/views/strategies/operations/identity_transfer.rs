//! Identity transfer operations form for strategy.

use dpp::{
    identity::accessors::IdentityGettersV0, platform_value::string_encoding::Encoding,
    prelude::Identifier,
};
use strategy_tests::{
    frequency::Frequency,
    operations::{IdentityTransferInfo, Operation, OperationType},
    Strategy,
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus,
        SelectInput, TextInput,
    },
};

use super::{ComposedInput, Field};

/// Identity Transfer Random Form Controller
pub(super) struct StrategyOpIdentityTransferRandomFormController {
    input: SelectInput<f64>,
    selected_strategy: String,
}

impl StrategyOpIdentityTransferRandomFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyOpIdentityTransferRandomFormController {
            input: SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1]), // Chance per iteration
            selected_strategy,
        }
    }
}

impl FormController for StrategyOpIdentityTransferRandomFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(chance_per_block) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::AddOperation {
                    strategy_name: self.selected_strategy.clone(),
                    operation: Operation {
                        op_type: OperationType::IdentityTransfer(None),
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
        "Identity transfer operation"
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

/// Identity Transfer Specific Form Controller
pub(super) struct StrategyOpIdentityTransferSpecificFormController {
    input: ComposedInput<(
        Field<SelectInput<String>>,
        Field<TextInput<DefaultTextInputParser<u64>>>,
        Field<SelectInput<f64>>,
    )>,
    selected_strategy_name: String,
    loaded_identity_id: String,
}

impl StrategyOpIdentityTransferSpecificFormController {
    pub(super) fn new(
        selected_strategy_name: String,
        selected_strategy: Strategy,
        loaded_identity_id: Option<String>,
    ) -> Self {
        let identity_id_strings: Vec<String> = selected_strategy
            .start_identities
            .hard_coded
            .iter()
            .map(|(identity, _)| identity.id().to_string(Encoding::Base58))
            .collect();

        Self {
            input: ComposedInput::new((
                // For now, sender is always loaded identity otherwise we need to get signer from sender
                // Field::new(
                //     "Select sender",
                //     SelectInput::new(identity_id_strings.clone()),
                // ),
                Field::new("Select recipient", SelectInput::new(identity_id_strings)), // TODO: Take out sender
                Field::new("Amount", TextInput::new("Minimum 300000")),
                Field::new(
                    "Chance per time unit",
                    SelectInput::new(vec![1.0, 0.9, 0.75, 0.5, 0.25, 0.1]),
                ),
            )),
            selected_strategy_name,
            loaded_identity_id: loaded_identity_id
                .expect("Need a loaded identity to create transfer"),
        }
    }
}

impl FormController for StrategyOpIdentityTransferSpecificFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(inputs) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::AddOperation {
                    strategy_name: self.selected_strategy_name.clone(),
                    operation: Operation {
                        op_type: OperationType::IdentityTransfer(Some(IdentityTransferInfo {
                            from: Identifier::from_string(
                                &self.loaded_identity_id,
                                Encoding::Base58,
                            )
                            .expect("Expected to convert string to Identifier"),
                            to: Identifier::from_string(&inputs.0, Encoding::Base58)
                                .expect("Expected to convert string to Identifier"),
                            amount: inputs.1,
                        })),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: Some(inputs.2),
                        },
                    },
                }),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity transfer operation"
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
        self.input.steps_number()
    }
}
