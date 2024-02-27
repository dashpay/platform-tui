//! Data contract update new fields operation form for strategy.

use rand::Rng;
use strategy_tests::{
    frequency::Frequency,
    operations::{
        DataContractUpdateAction::DataContractNewOptionalFields, DataContractUpdateOp, Operation,
        OperationType,
    },
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpContractUpdateNewFieldsFormController {
    input: ComposedInput<(Field<SelectInput<u16>>, Field<SelectInput<f64>>)>,
    selected_strategy: String,
}

impl StrategyOpContractUpdateNewFieldsFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyOpContractUpdateNewFieldsFormController {
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

// impl FormController for StrategyOpContractUpdateNewFieldsFormController {
//     fn on_event(&mut self, event: KeyEvent) -> FormStatus {
//         match self.input.on_event(event) {
//             InputStatus::Done((times_per_block, chance_per_block)) =>
// FormStatus::Done {                 task:
// Task::Strategy(StrategyTask::AddOperation {
// strategy_name: self.selected_strategy.clone(),                     operation:
// Operation {                         op_type:
// OperationType::ContractUpdate(DataContractUpdateOp {
// action: DataContractNewOptionalFields(
// 1..rand::thread_rng().gen_range(1..=u16::MAX),
// 1..rand::thread_rng().gen_range(1..=u16::MAX),                             ),
//                             contract,
//                             document_type,
//                         }),
//                         frequency: Frequency {
//                             times_per_block_range: 1..times_per_block+1,
//                             chance_per_block: Some(chance_per_block),
//                         },
//                     },
//                 }),
//                 block: false,
//             },
//             status => status.into(),
//         }
//     }

//     fn form_name(&self) -> &'static str {
//         "Contract update new fields operation"
//     }

//     fn step_view(&mut self, frame: &mut Frame, area: Rect) {
//         self.input.view(frame, area)
//     }

//     fn step_name(&self) -> &'static str {
//         self.input.step_name()
//     }

//     fn step_index(&self) -> u8 {
//         self.input.step_index()
//     }

//     fn steps_number(&self) -> u8 {
//         2
//     }
// }
