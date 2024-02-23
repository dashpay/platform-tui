//! Forms for operations management in strategy.

mod contract_create;
mod contract_update_doc_types;
mod contract_update_new_fields;
mod document;
mod identity_top_up;
mod identity_transfer;
mod identity_update;
mod identity_withdrawal;

use std::collections::BTreeMap;

use rs_sdk::platform::DataContract;
use strum::IntoEnumIterator;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use self::{
    contract_update_doc_types::StrategyOpContractUpdateDocTypesFormController,
    document::StrategyOpDocumentFormController,
    identity_top_up::StrategyOpIdentityTopUpFormController,
    identity_transfer::StrategyOpIdentityTransferFormController,
    identity_update::StrategyOpIdentityUpdateFormController,
    identity_withdrawal::StrategyOpIdentityWithdrawalFormController,
};
use crate::ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput};

#[derive(Debug, strum::Display, Clone, strum::EnumIter, Copy)]
enum OperationType {
    Document,
    IdentityTopUp,
    IdentityAddKeys,
    IdentityDisableKeys,
    // IdentityWithdrawal,
    IdentityTransfer,
    // ContractCreateRandom,
    ContractUpdateDocTypesRandom,
    // ContractUpdateFieldsRandom,
}

pub(super) struct StrategyAddOperationFormController {
    op_type_input: SelectInput<OperationType>,
    op_specific_form: Option<Box<dyn FormController>>,
    strategy_name: String,
    known_contracts: BTreeMap<String, DataContract>,
}

impl StrategyAddOperationFormController {
    pub(super) fn new(
        strategy_name: String,
        known_contracts: BTreeMap<String, DataContract>,
    ) -> Self {
        StrategyAddOperationFormController {
            op_type_input: SelectInput::new(OperationType::iter().collect()),
            op_specific_form: None,
            strategy_name,
            known_contracts,
        }
    }

    fn set_op_form(&mut self, op_type: OperationType) {
        self.op_specific_form = Some(match op_type {
            OperationType::Document => Box::new(StrategyOpDocumentFormController::new(
                self.strategy_name.clone(),
                self.known_contracts.clone(),
            )),
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
            // OperationType::IdentityWithdrawal => Box::new(
            //     StrategyOpIdentityWithdrawalFormController::new(self.strategy_name.clone()),
            // ),
            OperationType::IdentityTransfer => Box::new(
                StrategyOpIdentityTransferFormController::new(self.strategy_name.clone()),
            ),
            // OperationType::ContractCreateRandom => Box::new(
            //     StrategyOpContractCreateFormController::new(self.strategy_name.clone()),
            // ),
            OperationType::ContractUpdateDocTypesRandom => {
                Box::new(StrategyOpContractUpdateDocTypesFormController::new(
                    self.strategy_name.clone(),
                    self.known_contracts.clone(),
                ))
            }
            // OperationType::ContractUpdateFieldsRandom => Box::new(
            //     StrategyOpContractUpdateNewFieldsFormController::new(self.strategy_name.clone()),
            // ),
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
                status => status.into(),
            }
        }
    }

    fn form_name(&self) -> &'static str {
        if let Some(form) = &self.op_specific_form {
            form.form_name()
        } else {
            "Add strategy operation"
        }
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
