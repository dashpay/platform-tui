//! Strategy's contracts with updates form.

use std::collections::BTreeMap;

use dash_platform_sdk::platform::DataContract;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{ComposedInput, Field, FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub(super) struct StrategyContractsFormController {
    selected_strategy: String,
    known_contracts: BTreeMap<String, DataContract>,
    selected_contracts: Vec<String>,
    input: ComposedInput<(Field<SelectInput<String>>, Field<SelectInput<String>>)>,
}

impl StrategyContractsFormController {
    pub(super) fn new(
        selected_strategy: String,
        known_contracts: BTreeMap<String, DataContract>,
    ) -> Self {
        let contract_names: Vec<String> = known_contracts.keys().cloned().collect();
        StrategyContractsFormController {
            selected_strategy,
            known_contracts,
            selected_contracts: Vec::new(),
            input: ComposedInput::new((
                Field::new("Select Contract", SelectInput::new(contract_names)),
                Field::new(
                    "Add Another Contract? Note only compatible contract updates will actually \
                     work.",
                    SelectInput::new(vec!["Yes".to_string(), "No".to_string()]),
                ),
            )),
        }
    }
}

impl FormController for StrategyContractsFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((selected_contract, add_another_answer)) => {
                self.selected_contracts.push(selected_contract);

                if add_another_answer == "Yes" {
                    // Reset the input fields for another contract selection
                    let contract_names: Vec<String> =
                        self.known_contracts.keys().cloned().collect();
                    self.input = ComposedInput::new((
                        Field::new("Select Contract", SelectInput::new(contract_names)),
                        Field::new(
                            "Add Another Contract? Note only compatible contract updates will \
                             actually work.",
                            SelectInput::new(vec!["Yes".to_string(), "No".to_string()]),
                        ),
                    ));
                    FormStatus::Redraw
                } else {
                    FormStatus::Done {
                        task: Task::Strategy(StrategyTask::SetContractsWithUpdates(
                            self.selected_strategy.clone(),
                            self.selected_contracts.clone(),
                            self.known_contracts.clone(),
                        )),
                        block: false,
                    }
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Contract with updates for strategy"
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
