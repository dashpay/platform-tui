//! Start identities for strategy form.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{StrategyTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus, Input,
        InputStatus, SelectInput, TextInput,
    },
};

pub(super) struct StrategyStartIdentitiesFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<u8>>>,
        Field<TextInput<DefaultTextInputParser<u8>>>,
        Field<SelectInput<String>>,
    )>,
    selected_strategy: String,
}

impl StrategyStartIdentitiesFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        StrategyStartIdentitiesFormController {
            input: ComposedInput::new((
                Field::new(
                    "Number of identities",
                    TextInput::new("Enter a whole number"),
                ),
                Field::new(
                    "Keys per identity (min 3, max 32)",
                    TextInput::new("Enter a whole number"),
                ),
                Field::new(
                    "Add transfer key?",
                    SelectInput::new(vec!["Yes".to_string(), "No".to_string()]),
                ),
            )),
            selected_strategy,
        }
    }
}

impl FormController for StrategyStartIdentitiesFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((count, keys_count, add_transfer_key)) => {
                if add_transfer_key == "Yes" {
                    FormStatus::Done {
                        task: Task::Strategy(StrategyTask::SetStartIdentities {
                            strategy_name: self.selected_strategy.clone(),
                            count,
                            keys_count,
                            balance: 10_000_000,
                            add_transfer_key: true,
                        }),
                        block: false,
                    }
                } else {
                    FormStatus::Done {
                        task: Task::Strategy(StrategyTask::SetStartIdentities {
                            strategy_name: self.selected_strategy.clone(),
                            count,
                            keys_count,
                            balance: 10_000_000,
                            add_transfer_key: false,
                        }),
                        block: false,
                    }
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Start identities for strategy"
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
        3
    }
}

pub(super) struct StrategyStartIdentitiesBalanceFormController {
    input: TextInput<DefaultTextInputParser<f64>>,
    selected_strategy: String,
}

impl StrategyStartIdentitiesBalanceFormController {
    pub(super) fn new(selected_strategy: String) -> Self {
        Self {
            input: TextInput::new("Quantity (in Dash)"),
            selected_strategy,
        }
    }
}

impl FormController for StrategyStartIdentitiesBalanceFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(balance) => FormStatus::Done {
                task: Task::Strategy(StrategyTask::SetStartIdentitiesBalance(
                    self.selected_strategy.clone(),
                    (balance * 100000000.0) as u64,
                )),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Set start identities balances"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        ""
    }

    fn step_index(&self) -> u8 {
        1
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
