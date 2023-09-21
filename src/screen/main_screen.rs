//! First screen of the application.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Paragraph, Widget},
};

use crate::app::App;

use super::{identity_screen::IdentityScreen, Key, Screen};

#[derive(Debug, Clone)]
pub struct MainScreen {}

impl From<MainScreen> for Screen {
    fn from(value: MainScreen) -> Self {
        Screen::MainScreen(value)
    }
}

impl MainScreen {
    pub fn new() -> Self {
        MainScreen {}
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(
            "Welcome to Platform TUI!

Use keys listed in \"Commands\" section below to switch screens and/or toggle flags.

Some of them require signature and are disabled until an identity key is loaded.",
        )
        .render(area, buf)
    }

    pub fn keys(&self) -> &'static [Key] {
        [
            Key {
                key: 'q',
                description: "Quit",
            },
            Key {
                key: 'i',
                description: "Identities",
            },
            Key {
                key: 'c',
                description: "Data Contracts",
            },
            Key {
                key: 'd',
                description: "Documents",
            },
            Key {
                key: 's',
                description: "State Transitions",
            },
        ]
        .as_ref()
    }

    pub fn screen_name(&self) -> &'static str {
        "Main"
    }

    pub fn handle_key_event(self, app: &mut App, key_event: KeyEvent) -> Screen {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('q'),
                ..
            } => {
                app.quit();
                self.into()
            }
            KeyEvent {
                code: KeyCode::Char('i'),
                ..
            } => IdentityScreen::new().into(),
            _ => self.into(),
        }
    }

    pub fn breadcrumbs(&self) -> &'static [&'static str] {
        ["Main"].as_ref()
    }
}
