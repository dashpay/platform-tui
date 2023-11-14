//! Contracts views.

mod fetch_contract;

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::main::MainScreenController;
use crate::{
    backend::AppState,
    ui::{
        form::{Input, SelectInput},
        screen::{
            widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback,
            ScreenToggleKey,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Fetch system contract"),
];

pub(crate) struct ContractsScreenController {
    select: Option<SelectInput<String>>,
}

impl ContractsScreenController {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let select = if app_state.known_contracts.len() > 0 {
            Some(SelectInput::new(
                app_state.known_contracts.keys().cloned().collect(),
            ))
        } else {
            None
        };
        ContractsScreenController { select }
    }
}

impl ScreenController for ContractsScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(select) = &mut self.select {
            select.view(frame, area)
        } else {
            Info::new_fixed("No fetched data contracts").view(frame, area)
        }
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

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => {
                ScreenFeedback::PreviousScreen(Box::new(|_| Box::new(MainScreenController::new())))
            }
            _ => ScreenFeedback::None,
        }
    }
}
