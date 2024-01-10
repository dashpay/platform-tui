//! Start identities for strategy form.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyStartIdentitiesFormController {
    input: ComposedInput<(Field<SelectInput<u16>>, Field<SelectInput<u32>>)>,
    selected_strategy: String,
}

impl StrategyStartIdentitiesFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyStartIdentitiesFormController {
            input: ComposedInput::new((
                Field::new(
                    "Number of identities",
                    SelectInput::new(vec![1, 10, 100, 1000, 10000, u16::MAX]),
                ),
                Field::new(
                    "Keys per identity",
                    SelectInput::new(vec![2, 3, 4, 5, 10, 20, 32]),
                ),
            )),
            selected_strategy,
        }
    }
}

// impl FormController for StrategyStartIdentitiesFormController {
//     fn on_event(&mut self, event: KeyEvent) -> FormStatus {
//         match self.input.on_event(event) {
//             InputStatus::Done((count, key_count)) => FormStatus::Done {
//                 task: Task::Strategy(StrategyTask::SetStartIdentities {
//                     strategy_name: self.selected_strategy.clone(),
//                     count,
//                     key_count,
//                 }),
//                 block: true,
//             },
//             status => status.into(),
//         }
//     }

//     fn form_name(&self) -> &'static str {
//         "Identity inserts for strategy"
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
