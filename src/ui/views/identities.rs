//! UI definitions related to identities.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    Frame,
};

use crate::{
    backend::Task,
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, TextInput},
        screen::{ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey},
        views::main::MainScreenController,
    },
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("i", "Get Identity by ID"),
];

pub(crate) struct IdentitiesScreenController {
    toggle_keys: [ScreenToggleKey; 1],
}

impl IdentitiesScreenController {
    pub(crate) fn new() -> Self {
        IdentitiesScreenController {
            toggle_keys: [ScreenToggleKey::new("p", "with proof")],
        }
    }
}

impl ScreenController for IdentitiesScreenController {
    fn name(&self) -> &'static str {
        "Identities"
    }

    fn init_text(&self) -> &'static str {
        "Identity management commands"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        self.toggle_keys.as_ref()
    }

    fn on_event(&mut self, key_event: KeyEvent) -> ScreenFeedback {
        match key_event {
            KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            } => ScreenFeedback::PreviousScreen(Box::new(MainScreenController)),
            KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            } => ScreenFeedback::Form(Box::new(GetIdentityByIdFormController::new())),
            KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            } => {
                self.toggle_keys[0].toggle = !self.toggle_keys[0].toggle;
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
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
            InputStatus::Done(value) => FormStatus::Done(Task::FetchIdentityById(value)),
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
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
