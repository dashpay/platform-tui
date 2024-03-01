//! Data contract update doc types operation form for strategy.

use std::{cmp::min, collections::BTreeMap};

use dpp::data_contract::{
    document_type::v0::random_document_type::{
        FieldMinMaxBounds, FieldTypeWeights, RandomDocumentTypeParameters,
    },
    DataContract,
};
use rand::Rng;
use strategy_tests::{
    frequency::Frequency,
    operations::{
        DataContractUpdateAction::DataContractNewDocumentTypes, DataContractUpdateOp, Operation,
        OperationType,
    },
};
use tracing::error;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyOpContractUpdateDocTypesFormController {
    input: ComposedInput<(
        Field<SelectInput<String>>,
        Field<SelectInput<u16>>,
        Field<SelectInput<f64>>,
    )>,
    selected_strategy: String,
    known_contracts: BTreeMap<String, DataContract>,
}

impl StrategyOpContractUpdateDocTypesFormController {
    pub(super) fn new(
        selected_strategy: String,
        known_contracts: BTreeMap<String, DataContract>,
    ) -> Self {
        StrategyOpContractUpdateDocTypesFormController {
            input: ComposedInput::new((
                Field::new(
                    "Contract",
                    SelectInput::new(known_contracts.keys().cloned().collect()),
                ),
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
            known_contracts,
        }
    }
}

impl FormController for StrategyOpContractUpdateDocTypesFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        let random_number1 = rand::thread_rng().gen_range(3..=50);
        let random_number2 = rand::thread_rng().gen_range(3..=50);
        let random_number3 = rand::thread_rng().gen::<i64>() - 1000000;

        let random_doc_type_parameters = RandomDocumentTypeParameters {
            new_fields_optional_count_range: 1..random_number1,
            new_fields_required_count_range: 1..random_number2,
            new_indexes_count_range: 1..rand::thread_rng().gen_range(2..=10),
            field_weights: FieldTypeWeights {
                string_weight: rand::thread_rng().gen_range(1..=100),
                float_weight: rand::thread_rng().gen_range(1..=100),
                integer_weight: rand::thread_rng().gen_range(1..=100),
                date_weight: rand::thread_rng().gen_range(1..=100),
                boolean_weight: rand::thread_rng().gen_range(1..=100),
                byte_array_weight: rand::thread_rng().gen_range(1..=100),
            },
            field_bounds: FieldMinMaxBounds {
                string_min_len: 1..10,
                string_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                string_max_len: 10..63,
                string_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                integer_min: 1..10,
                integer_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                integer_max: 10..10000,
                integer_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                float_min: 0.1..10.0,
                float_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                float_max: 10.0..1000.0,
                float_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                date_min: random_number3,
                date_max: random_number3 + 1000000,
                byte_array_min_len: 1..10,
                byte_array_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                byte_array_max_len: 10..255,
                byte_array_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
            },
            keep_history_chance: rand::thread_rng().gen_range(0.01..=1.0),
            documents_mutable_chance: rand::thread_rng().gen_range(0.01..=1.0),
        };

        match self.input.on_event(event) {
            InputStatus::Done((contract_name, times_per_block, chance_per_block)) => {
                // Retrieve the DataContract object by its name
                if let Some(contract) = self.known_contracts.get(&contract_name) {
                    FormStatus::Done {
                        task: Task::Strategy(StrategyTask::AddOperation {
                            strategy_name: self.selected_strategy.clone(),
                            operation: Operation {
                                op_type: OperationType::ContractUpdate(DataContractUpdateOp {
                                    action: DataContractNewDocumentTypes(
                                        random_doc_type_parameters,
                                    ),
                                    contract: contract.clone(),
                                    document_type: None,
                                }),
                                frequency: Frequency {
                                    times_per_block_range: 1..times_per_block + 1,
                                    chance_per_block: Some(chance_per_block),
                                },
                            },
                        }),
                        block: false,
                    }
                } else {
                    error!("No contract in known_contracts with that name");
                    FormStatus::None
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Contract update doc types operation"
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
        2
    }
}
