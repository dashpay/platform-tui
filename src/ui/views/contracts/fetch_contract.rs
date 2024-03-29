//! Contract fetching screen module.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{BackendEvent, Task},
    ui::{form::{parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus, TextInput}, screen::{
        utils::impl_builder_no_args, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
    }},
    Event,
};
use crate::ui::views::contracts::ContractTask::FetchContract;

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Contracts"),
    ScreenCommandKey::new("f", "Fetch contract by ID"),
];

pub(crate) struct FetchContractScreenController {
    info: Info,
}

impl_builder_no_args!(FetchContractScreenController);

impl FetchContractScreenController {
    pub(crate) fn new() -> Self {
        Self {
            info: Info::new_fixed("Fetch contracts"),
        }
    }
}

impl ScreenController for FetchContractScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }

    fn name(&self) -> &'static str {
        "Contracts"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,

            Event::Key(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(GetContractByIdFormController::new())),

            Event::Backend(
                BackendEvent::TaskCompleted {
                    task: Task::Contract(_),
                    execution_result,
                }
                | BackendEvent::TaskCompletedStateChange {
                    task: Task::Contract(_),
                    execution_result,
                    ..
                },
            ) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

pub(crate) struct GetContractByIdFormController {
    input: TextInput<DefaultTextInputParser<String>>, // TODO: b58 parser
}

impl GetContractByIdFormController {
    fn new() -> Self {
        Self {
            input: TextInput::new("base58 id"),
        }
    }
}

impl FormController for GetContractByIdFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(value) => FormStatus::Done {
                task: Task::Contract(FetchContract(value)),
                block: true,
            },
            status => status.into(),
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: tuirealm::tui::prelude::Rect) {
        self.input.view(frame, area);
    }

    fn form_name(&self) -> &'static str {
        "Get Contract by ID"
    }

    fn step_name(&self) -> &'static str {
        "Base 58 ID"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
