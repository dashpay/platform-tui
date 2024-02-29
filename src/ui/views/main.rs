//! The view a user sees on application start.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::{contracts::ContractsScreenController, wallet::WalletScreenController};
use crate::ui::views::strategies::StrategiesScreenController;
use crate::{
    ui::{
        screen::{
            utils::impl_builder_no_args, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
        views::{
            identities::IdentitiesScreenController,
            platform_info::PlatformInfoScreenController,
            //            strategies::StrategiesScreenController,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 7] = [
    ScreenCommandKey::new("q", "Quit"),
    ScreenCommandKey::new("i", "Identities"),
    ScreenCommandKey::new("c", "Contracts"),
    ScreenCommandKey::new("s", "Strategies"),
    ScreenCommandKey::new("w", "Wallet"),
    ScreenCommandKey::new("v", "Version Upgrade"),
    ScreenCommandKey::new("p", "Platform information"),
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
Use q to go back from completion list or once again to leave input at all.
"#,
            ),
        }
    }
}

impl_builder_no_args!(MainScreenController);

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

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Quit,
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(IdentitiesScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(StrategiesScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('w'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(WalletScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(ContractsScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::None,
            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(PlatformInfoScreenController::builder()),
            _ => ScreenFeedback::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }
}
