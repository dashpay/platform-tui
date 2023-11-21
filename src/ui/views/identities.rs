//! UI definitions related to identities.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    backend::{BackendEvent, Task},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, TextInput},
        screen::{
            utils::impl_builder_no_args, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
        views::main::MainScreenController,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("i", "Get Identity by ID"),
];

pub(crate) struct IdentitiesScreenController {
    toggle_keys: [ScreenToggleKey; 1],
    info: Info,
}

impl_builder_no_args!(IdentitiesScreenController);

impl IdentitiesScreenController {
    pub(crate) fn new() -> Self {
        IdentitiesScreenController {
            toggle_keys: [ScreenToggleKey::new("p", "with proof")],
            info: Info::new_fixed("Identity management commands"),
        }
    }
}

impl ScreenController for IdentitiesScreenController {
    fn name(&self) -> &'static str {
        "Identities"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        self.toggle_keys.as_ref()
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen(MainScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(GetIdentityByIdFormController::new())),
            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.toggle_keys[0].toggle = !self.toggle_keys[0].toggle;
                ScreenFeedback::Redraw
            }
            Event::Key(k) => {
                let redraw_info = self.info.on_event(k);
                if redraw_info {
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }

            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::FetchIdentityById(..),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}

pub(crate) struct GetIdentityByIdFormController {
    input: TextInput,
}

impl GetIdentityByIdFormController {
    fn new() -> Self {
        GetIdentityByIdFormController {
            input: TextInput::new("base58 id"),
        }
    }
}

impl FormController for GetIdentityByIdFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(value) => FormStatus::Done {
                task: Task::FetchIdentityById(value, false),
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: tuirealm::tui::prelude::Rect) {
        self.input.view(frame, area);
    }

    fn form_name(&self) -> &'static str {
        "Get identity by ID"
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
