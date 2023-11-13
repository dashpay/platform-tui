//! Forms for operations management in strategy.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::Task,
    ui::form::{
        ComposedInput, Field, Form, FormController, FormStatus, Input, InputStatus, SelectInput,
    },
};

#[derive(Debug, strum::Display, Clone)]
enum OperationType {
    IdentityTopUp,
    IdentityUpdate,
    IdentityWithdrawal,
    IdentityTransfer,
    ContractCreate,
    ContractUpdate,
    Document,
}

pub(super) struct StrategyAddOperationFormController {
    op_type_input: SelectInput<OperationType>,
    op_specific_form: Option<Form<Box<dyn FormController>>>,
}

impl FormController for StrategyAddOperationFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        if let Some(form) = self.op_specific_form {
            form.on_event(event)
        }
    }

    fn form_name(&self) -> &'static str {
        todo!()
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        todo!()
    }

    fn step_name(&self) -> &'static str {
        todo!()
    }

    fn step_index(&self) -> u8 {
        todo!()
    }

    fn steps_number(&self) -> u8 {
        todo!()
    }
}
