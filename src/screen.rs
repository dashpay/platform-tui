//! Representation of different screens of the app.

mod identity_screen;
mod main_screen;

use crossterm::event::KeyEvent;
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::Widget,
};

use crate::app::App;
pub use main_screen::MainScreen;

use self::identity_screen::IdentityScreen;

#[derive(Debug)]
pub struct Key {
    pub key: char,
    pub description: &'static str,
    // TODO can be a toggle
}

// Unfortunately it cannot be made as a trait because widget rendering consumes it
// and trait object refuses to work in such case
#[derive(Debug, Clone)]
pub enum Screen {
    MainScreen(MainScreen),
    IdentityScreen(IdentityScreen),
}

impl Default for Screen {
    fn default() -> Self {
        MainScreen::new().into()
    }
}

impl Widget for Screen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Screen::MainScreen(s) => s.render(area, buf),
            Screen::IdentityScreen(s) => s.render(area, buf),
        }
    }
}

impl Screen {
    pub fn keys(&self) -> &'static [Key] {
        match self {
            Screen::MainScreen(s) => s.keys(),
            Screen::IdentityScreen(s) => s.keys(),
        }
    }

    pub fn screen_name(&self) -> &'static str {
        match self {
            Screen::MainScreen(s) => s.screen_name(),
            Screen::IdentityScreen(s) => s.screen_name(),
        }
    }

    pub fn breadcrumbs(&self) -> &'static [&'static str] {
        match self {
            Screen::MainScreen(s) => s.breadcrumbs(),
            Screen::IdentityScreen(s) => s.breadcrumbs(),
        }
    }

    pub fn handle_key_event(self, app: &mut App, key_event: KeyEvent) -> Screen {
        match self {
            Screen::MainScreen(s) => s.handle_key_event(app, key_event),
            Screen::IdentityScreen(s) => s.handle_key_event(app, key_event),
        }
    }
}
