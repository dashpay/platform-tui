use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Paragraph, Widget},
};

use crate::app::App;

use super::{Key, MainScreen, Screen};

#[derive(Debug, Clone)]
pub struct IdentityScreen {}

impl IdentityScreen {
    pub fn new() -> Self {
        IdentityScreen {}
    }
}

impl From<IdentityScreen> for Screen {
    fn from(value: IdentityScreen) -> Self {
        Screen::IdentityScreen(value)
    }
}

impl IdentityScreen {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Identity queries.").render(area, buf)
    }

    pub fn keys(&self) -> &'static [Key] {
        [Key {
            key: 'q',
            description: "Back",
        }]
        .as_ref()
    }

    pub fn screen_name(&self) -> &'static str {
        "Identity"
    }

    pub fn breadcrumbs(&self) -> &'static [&'static str] {
        ["Main", "Identity"].as_ref()
    }

    pub fn handle_key_event(self, app: &mut App, key_event: KeyEvent) -> Screen {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('q'),
                ..
            } => MainScreen::new().into(),
            _ => self.into(),
        }
    }
}
