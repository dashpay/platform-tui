//! Platform invo views.

use std::fmt::Display;

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{
        platform_info::PlatformInfoTask::{
            FetchCurrentEpochInfo, FetchCurrentVersionVotingState, FetchSpecificEpochInfo,
        },
        AppState, BackendEvent, Task,
    },
    ui::{
        form::{
            parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus,
            TextInput,
        },
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 4] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("c", "Fetch current Platform epoch info"),
    ScreenCommandKey::new("i", "Fetch previous Platform epoch info"),
    ScreenCommandKey::new("v", "Current version voting"),
];

pub(crate) struct PlatformInfoScreenController {
    info: Info,
}

impl_builder!(PlatformInfoScreenController);

impl PlatformInfoScreenController {
    pub(crate) async fn new(_app_state: &AppState) -> Self {
        PlatformInfoScreenController {
            info: Info::new_fixed("Identity management commands"),
        }
    }
}

impl ScreenController for PlatformInfoScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }

    fn name(&self) -> &'static str {
        "Platform Information"
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
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentEpochInfo),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::PlatformInfo(FetchCurrentVersionVotingState),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(EpochNumberChooserFormController::new())),

            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::PlatformInfo(_),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

struct EpochNumberChooserFormController {
    input: TextInput<DefaultTextInputParser<u16>>,
}

impl EpochNumberChooserFormController {
    fn new() -> Self {
        EpochNumberChooserFormController {
            input: TextInput::new("Epoch number"),
        }
    }
}

impl FormController for EpochNumberChooserFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(epoch) => FormStatus::Done {
                task: Task::PlatformInfo(FetchSpecificEpochInfo(epoch)),
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Epoch number"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Input epoch number"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
