//! Forms for strategy operations related to document operations.

use std::collections::BTreeMap;

use dpp::data_contract::{
    accessors::v0::DataContractV0Getters,
    document_type::{
        random_document::{DocumentFieldFillSize, DocumentFieldFillType},
        DocumentType,
    },
};
use rs_sdk::platform::DataContract;
use strategy_tests::{
    frequency::Frequency,
    operations::{DocumentAction, DocumentOp, Operation, OperationType},
};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyContractNames, StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpDocumentFormController {
    input: ComposedInput<(
        Field<SelectInput<String>>,
        Field<SelectInput<String>>,
        Field<SelectInput<u16>>,
        Field<SelectInput<f64>>,
    )>,
    selected_strategy_name: String,
    known_contracts: BTreeMap<String, DataContract>,
    supporting_contracts: BTreeMap<String, DataContract>,
    document_types: BTreeMap<String, DocumentType>,
    strategy_contract_names: StrategyContractNames,
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
            if strategy_contract_names.iter().any(|(name, _)| name == supporting_contract_name) {
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
        let contract_names: Vec<String> = contract_names.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

        let action_types = vec![
            "Insert Random".to_string(),
            // "Delete".to_string(),
            // "Replace".to_string(),
        ];

        Self {
            input: ComposedInput::new((
                Field::new("Select Contract", SelectInput::new(contract_names)),
                Field::new("Select Action", SelectInput::new(action_types)),
                Field::new(
                    "Times per block",
                    SelectInput::new(vec![1, 2, 5, 10, 20, 24]),
                ),
                Field::new(
                    "Chance per block",
                    SelectInput::new(vec![1.0, 0.75, 0.5, 0.25, 0.1]),
                ),
            )),
            selected_strategy_name,
            known_contracts,
            supporting_contracts,
            document_types: BTreeMap::new(),
            strategy_contract_names,
        }
    }
}

impl FormController for StrategyOpDocumentFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((contract_name, action_type, times_per_block, chance_per_block)) => {
                let selected_contract = self.known_contracts.get(&contract_name)
                    .or_else(|| self.supporting_contracts.get(&contract_name))
                    .expect("Contract name not found in known_contracts or supporting_contracts.");

                let document_types = selected_contract.document_types();
                self.document_types = document_types.clone();

                // To-do: let the user select the document type
                // Pretty sure this just selects the first document type every time
                let selected_document_type = self.document_types.values().next().unwrap().clone();

                let action = match action_type.as_ref() {
                    "Insert Random" => DocumentAction::DocumentActionInsertRandom(
                        DocumentFieldFillType::FillIfNotRequired,
                        DocumentFieldFillSize::AnyDocumentFillSize,
                    ),
                    // "Delete" => DocumentAction::DocumentActionDelete,
                    // "Replace" => DocumentAction::DocumentActionReplace,
                    _ => panic!("Invalid action type"),
                };

                FormStatus::Done {
                    task: Task::Strategy(StrategyTask::AddOperation {
                        strategy_name: self.selected_strategy_name.clone(),
                        operation: Operation {
                            op_type: OperationType::Document(DocumentOp {
                                contract: selected_contract.clone(),
                                document_type: selected_document_type,
                                action: action.clone(),
                            }),
                            frequency: Frequency {
                                times_per_block_range: 1..times_per_block + 1,
                                chance_per_block: Some(chance_per_block),
                            },
                        },
                    }),
                    block: false,
                }
            }
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Document operations"
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
        4
    }
}
