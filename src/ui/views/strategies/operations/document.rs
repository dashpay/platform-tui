//! Forms for strategy operations related to document operations.

use std::collections::BTreeMap;

use dash_sdk::platform::DataContract;
use dpp::data_contract::{
    accessors::v0::DataContractV0Getters,
    document_type::{
        random_document::{DocumentFieldFillSize, DocumentFieldFillType},
        DocumentType,
    },
};
use itertools::Itertools;
use strategy_tests::{
    frequency::Frequency,
    operations::{DocumentAction, DocumentOp, Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyContractNames, StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus, Input,
        InputStatus, SelectInput, TextInput,
    },
};

// Select a contract, then move to a new form with the document types
pub(super) struct StrategyOpDocumentFormController {
    input: SelectInput<String>,
    contract_specific_form: Option<Box<dyn FormController>>,
    strategy_name: String,
    known_contracts: BTreeMap<String, DataContract>,
    supporting_contracts: BTreeMap<String, DataContract>,
}

impl StrategyOpDocumentFormController {
    pub(super) fn new(
        selected_strategy_name: String,
        known_contracts: BTreeMap<String, DataContract>,
        supporting_contracts: BTreeMap<String, DataContract>,
        strategy_contract_names: StrategyContractNames,
    ) -> Self {
        // Collect known_contracts and supporting_contracts names for the form
        let mut contract_names: Vec<String> = known_contracts.keys().cloned().collect();

        // Add only supporting contracts that are also in strategy_contract_names
        for (supporting_contract_name, _) in supporting_contracts.iter() {
            if strategy_contract_names
                .iter()
                .any(|(name, _)| name == supporting_contract_name)
            {
                contract_names.push(supporting_contract_name.clone());
            }
        }

        // Flatten the nested structure of strategy_contract_names and add to contract_names
        for (_, optional_map) in strategy_contract_names.iter() {
            if let Some(map) = optional_map {
                for (_, contract_name) in map.iter() {
                    contract_names.push(contract_name.clone());
                }
            }
        }

        // Remove duplicates
        let contract_names: Vec<String> = contract_names
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Self {
            input: SelectInput::new(contract_names),
            contract_specific_form: None,
            strategy_name: selected_strategy_name,
            known_contracts,
            supporting_contracts,
        }
    }

    fn set_contract_form(&mut self, contract: DataContract, document_types: Vec<String>) {
        self.contract_specific_form = Some(Box::new(DocumentTypeFormController::new(
            self.strategy_name.clone(),
            contract,
            document_types,
        )));
    }
}

impl FormController for StrategyOpDocumentFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        if let Some(form) = &mut self.contract_specific_form {
            form.on_event(event)
        } else {
            match self.input.on_event(event) {
                InputStatus::Done(contract_name) => {
                    let selected_contract = self
                        .known_contracts
                        .get(&contract_name)
                        .or_else(|| self.supporting_contracts.get(&contract_name))
                        .expect(
                            "Contract name not found in known_contracts or supporting_contracts.",
                        );

                    let document_types = selected_contract
                        .document_types()
                        .iter()
                        .map(|(name, _)| name.clone())
                        .collect_vec();

                    self.set_contract_form(selected_contract.clone(), document_types);
                    FormStatus::Redraw
                }
                status => status.into(),
            }
        }
    }

    fn form_name(&self) -> &'static str {
        if let Some(form) = &self.contract_specific_form {
            form.form_name()
        } else {
            "Add documents operation"
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(form) = &mut self.contract_specific_form {
            form.step_view(frame, area)
        } else {
            self.input.view(frame, area)
        }
    }

    fn step_name(&self) -> &'static str {
        if let Some(form) = &self.contract_specific_form {
            form.step_name()
        } else {
            "Select contract"
        }
    }

    fn step_index(&self) -> u8 {
        if let Some(form) = &self.contract_specific_form {
            form.step_index()
        } else {
            0
        }
    }

    fn steps_number(&self) -> u8 {
        if let Some(form) = &self.contract_specific_form {
            form.steps_number()
        } else {
            1
        }
    }
}

pub(super) struct DocumentTypeFormController {
    input: ComposedInput<(
        Field<SelectInput<String>>,                    // Document types
        Field<SelectInput<String>>,                    // Operation type
        Field<TextInput<DefaultTextInputParser<u16>>>, // Times per block
        Field<SelectInput<f64>>,                       // Chance per block
    )>,
    selected_strategy_name: String,
    selected_contract: DataContract,
    document_insert_form: Option<Box<dyn FormController>>,
}

impl DocumentTypeFormController {
    pub(super) fn new(
        selected_strategy_name: String,
        selected_contract: DataContract,
        document_types: Vec<String>,
    ) -> Self {
        Self {
            input: ComposedInput::new((
                Field::new("Select Document Type", SelectInput::new(document_types)),
                Field::new(
                    "Select Operation Type",
                    SelectInput::new(vec![
                        "DocumentInsertRandom".to_string(),
                        "DocumentDeleteRandom".to_string(),
                    ]),
                ),
                Field::new("Times per block", TextInput::new("Enter a whole number")),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.75, 0.5, 0.25, 0.1]),
                ),
            )),
            selected_strategy_name,
            selected_contract,
            document_insert_form: None,
        }
    }

    fn set_document_insert_form(
        &mut self,
        contract: DataContract,
        document_type: DocumentType,
        frequency: Frequency,
    ) {
        self.document_insert_form = Some(Box::new(DocumentInsertFormController::new(
            self.selected_strategy_name.clone(),
            contract,
            document_type,
            frequency,
        )));
    }
}

impl FormController for DocumentTypeFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        if let Some(form) = &mut self.document_insert_form {
            form.on_event(event)
        } else {
            match self.input.on_event(event) {
                InputStatus::Done((
                    document_type,
                    operation_type,
                    times_per_block,
                    chance_per_block,
                )) => {
                    let action = match operation_type.as_str() {
                        "DocumentInsertRandom" => DocumentAction::DocumentActionInsertRandom(
                            DocumentFieldFillType::DoNotFillIfNotRequired, // dummy value
                            DocumentFieldFillSize::MinDocumentFillSize,    // dummy value
                        ),
                        "DocumentDeleteRandom" => DocumentAction::DocumentActionDelete,
                        _ => panic!("Unrecognized string for Operation Type in Document Operation"),
                    };

                    match action {
                        DocumentAction::DocumentActionInsertRandom(_, _) => {
                            self.set_document_insert_form(
                                self.selected_contract.clone(),
                                self.selected_contract
                                    .document_type_cloned_for_name(&document_type)
                                    .expect("Expected the document type to be there"),
                                Frequency {
                                    times_per_block_range: times_per_block..times_per_block + 1,
                                    chance_per_block: Some(chance_per_block),
                                },
                            );
                            FormStatus::Redraw
                        }
                        DocumentAction::DocumentActionDelete => FormStatus::Done {
                            task: Task::Strategy(StrategyTask::AddOperation {
                                strategy_name: self.selected_strategy_name.clone(),
                                operation: Operation {
                                    op_type: OperationType::Document(DocumentOp {
                                        contract: self.selected_contract.clone(),
                                        document_type: self
                                            .selected_contract
                                            .document_type_cloned_for_name(&document_type)
                                            .expect("Expected the document type to be there"),
                                        action,
                                    }),
                                    frequency: Frequency {
                                        times_per_block_range: times_per_block..times_per_block + 1,
                                        chance_per_block: Some(chance_per_block),
                                    },
                                },
                            }),
                            block: false,
                        },
                        DocumentAction::DocumentActionInsertSpecific(_, _, _, _) => todo!(),
                        DocumentAction::DocumentActionReplaceRandom => todo!(),
                        DocumentAction::DocumentActionTransferRandom => todo!(),
                    }
                }
                status => status.into(),
            }
        }
    }

    fn form_name(&self) -> &'static str {
        if let Some(form) = &self.document_insert_form {
            form.form_name()
        } else {
            "Document type, operation, and frequency"
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(form) = &mut self.document_insert_form {
            form.step_view(frame, area)
        } else {
            self.input.view(frame, area)
        }
    }

    fn step_name(&self) -> &'static str {
        if let Some(form) = &self.document_insert_form {
            form.step_name()
        } else {
            self.input.step_name()
        }
    }

    fn step_index(&self) -> u8 {
        if let Some(form) = &self.document_insert_form {
            form.step_index()
        } else {
            self.input.step_index()
        }
    }

    fn steps_number(&self) -> u8 {
        if let Some(form) = &self.document_insert_form {
            form.steps_number()
        } else {
            self.input.steps_number()
        }
    }
}

pub(super) struct DocumentInsertFormController {
    input: ComposedInput<(
        Field<SelectInput<String>>, // Fill size
        Field<SelectInput<String>>, // Fill type
    )>,
    selected_strategy_name: String,
    selected_contract: DataContract,
    selected_document_type: DocumentType,
    frequency: Frequency,
}

impl DocumentInsertFormController {
    pub(super) fn new(
        selected_strategy_name: String,
        selected_contract: DataContract,
        selected_document_type: DocumentType,
        frequency: Frequency,
    ) -> Self {
        Self {
            input: ComposedInput::new((
                Field::new(
                    "How much data to populate the document with?",
                    SelectInput::new(vec![
                        "Minimum".to_string(),
                        "Maximum".to_string(),
                        "Random".to_string(),
                    ]),
                ),
                Field::new(
                    "Populate not-required fields?",
                    SelectInput::new(vec!["No".to_string(), "Yes".to_string()]),
                ),
            )),
            selected_strategy_name,
            selected_contract,
            selected_document_type,
            frequency,
        }
    }
}

impl FormController for DocumentInsertFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((fill_size_string, fill_type_string)) => {
                let fill_size = match &fill_size_string as &str {
                    "Minimum" => DocumentFieldFillSize::MinDocumentFillSize,
                    "Maximum" => DocumentFieldFillSize::MaxDocumentFillSize,
                    "Random" => DocumentFieldFillSize::AnyDocumentFillSize,
                    _ => {
                        tracing::error!("Fill size string invalid in document creation. Setting to AnyDocumentFillSize.");
                        DocumentFieldFillSize::AnyDocumentFillSize
                    }
                };
                let fill_type = match &fill_type_string as &str {
                    "Yes" => DocumentFieldFillType::FillIfNotRequired,
                    "No" => DocumentFieldFillType::DoNotFillIfNotRequired,
                    _ => {
                        tracing::error!("Fill type string invalid in document creation. Setting to DoNotFillIfNotRequired.");
                        DocumentFieldFillType::DoNotFillIfNotRequired
                    }
                };

                let action = DocumentAction::DocumentActionInsertRandom(fill_type, fill_size);

                FormStatus::Done {
                    task: Task::Strategy(StrategyTask::AddOperation {
                        strategy_name: self.selected_strategy_name.clone(),
                        operation: Operation {
                            op_type: OperationType::Document(DocumentOp {
                                contract: self.selected_contract.clone(),
                                document_type: self.selected_document_type.clone(),
                                action,
                            }),
                            frequency: self.frequency.clone(),
                        },
                    }),
                    block: false,
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Document insert random options"
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
