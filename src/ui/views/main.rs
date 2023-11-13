//! The view a user sees on application start.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use crate::{
    ui::{
        screen::{
            widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback,
            ScreenToggleKey,
        },
        views::{identities::IdentitiesScreenController, strategies::StrategiesScreenController},
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 6] = [
    ScreenCommandKey::new("q", "Quit"),
    ScreenCommandKey::new("i", "Identities"),
    ScreenCommandKey::new("c", "Contracts"),
    ScreenCommandKey::new("s", "Strategies"),
    ScreenCommandKey::new("w", "Wallet"),
    ScreenCommandKey::new("v", "Version Upgrade"),
];

pub(crate) struct MainScreenController {
    info: Info,
}

impl MainScreenController {
    pub(crate) fn new() -> Self {
        MainScreenController {
            info: Info::new_fixed(
                r#"Welcome to Platform TUI!

Use keys listed in the section below to switch screens and execute commands.
Some of them require signature and are disabled until an identity key is loaded.

Italics are used to mark flags.
Bold italics are flags that are enabled.

Text inputs with completions support both arrows and Ctrl+n / Ctrl+p keys for selection.
Use Ctrl+q to go back from completion list or once again to leave input at all.
"#,
            ),
        }
    }
}

impl ScreenController for MainScreenController {
    fn name(&self) -> &'static str {
        "Main menu"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        [].as_ref()
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Quit,
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => {
                ScreenFeedback::NextScreen(Box::new(
                    |_| Box::new(IdentitiesScreenController::new()),
                ))
            }
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::None,
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(Box::new(|app_state| {
                Box::new(StrategiesScreenController::new(app_state))
            })),
            Event::Key(KeyEvent {
                code: Key::Char('w'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::None,
            Event::Key(KeyEvent {
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::None,
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}
