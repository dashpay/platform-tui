//! Forms for operations management in strategy.

mod identity_top_up;
mod identity_update;

use strum::IntoEnumIterator;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use self::{
    identity_top_up::StrategyOpIdentityTopUpFormController,
    identity_update::StrategyOpIdentityUpdateFormController,
};
use crate::ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput};

#[derive(Debug, strum::Display, Clone, strum::EnumIter, Copy)]
enum OperationType {
    IdentityTopUp,
    IdentityAddKeys,
    IdentityDisableKeys,
    IdentityWithdrawal,
    IdentityTransfer,
    ContractCreate,
    ContractUpdate,
    Document,
}

pub(super) struct StrategyAddOperationFormController {
    op_type_input: SelectInput<OperationType>,
    op_specific_form: Option<Box<dyn FormController>>,
    strategy_name: String,
}

impl StrategyAddOperationFormController {
    pub(super) fn new(strategy_name: String) -> Self {
        StrategyAddOperationFormController {
            op_type_input: SelectInput::new(OperationType::iter().collect()),
            op_specific_form: None,
            strategy_name,
        }
    }

    fn set_op_form(&mut self, op_type: OperationType) {
        self.op_specific_form = Some(match op_type {
            OperationType::IdentityTopUp => Box::new(StrategyOpIdentityTopUpFormController::new(
                self.strategy_name.clone(),
            )),
            OperationType::IdentityAddKeys => {
                Box::new(StrategyOpIdentityUpdateFormController::new(
                    self.strategy_name.clone(),
                    identity_update::KeyUpdateOp::AddKeys,
                ))
            }
            OperationType::IdentityDisableKeys => {
                Box::new(StrategyOpIdentityUpdateFormController::new(
                    self.strategy_name.clone(),
                    identity_update::KeyUpdateOp::DisableKeys,
                ))
            }
            OperationType::IdentityWithdrawal => todo!(),
            OperationType::IdentityTransfer => todo!(),
            OperationType::ContractCreate => todo!(),
            OperationType::ContractUpdate => todo!(),
            OperationType::Document => todo!(),
        });
    }
}

impl FormController for StrategyAddOperationFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        if let Some(form) = &mut self.op_specific_form {
            form.on_event(event)
        } else {
            match self.op_type_input.on_event(event) {
                InputStatus::Done(op_type) => {
                    self.set_op_form(op_type);
                    FormStatus::Redraw
                }
                InputStatus::Redraw => FormStatus::Redraw,
                InputStatus::None => FormStatus::None,
            }
        }
    }

    fn form_name(&self) -> &'static str {
        "Add strategy operation"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(form) = &mut self.op_specific_form {
            form.step_view(frame, area)
        } else {
            self.op_type_input.view(frame, area)
        }
    }

    fn step_name(&self) -> &'static str {
        if let Some(form) = &self.op_specific_form {
            form.step_name()
        } else {
            "Select operation"
        }
    }

    fn step_index(&self) -> u8 {
        if let Some(form) = &self.op_specific_form {
            form.step_index()
        } else {
            0
        }
    }

    fn steps_number(&self) -> u8 {
        if let Some(form) = &self.op_specific_form {
            form.steps_number()
        } else {
            1
        }
    }
}
