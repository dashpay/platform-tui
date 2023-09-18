//! Representation of different screens of the app.

mod main_screen;
pub use main_screen::MainScreen;
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Paragraph, Widget},
};

#[derive(Debug)]
pub struct Key {
    key: char,
    description: &'static str,
}

#[derive(Debug, Clone)]
pub enum Screen {
    MainScreen(MainScreen),
}

impl Default for Screen {
    fn default() -> Self {
        Screen::MainScreen(MainScreen::new())
    }
}

impl Widget for Screen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Screen::MainScreen(x) => Paragraph::new("This is a main screen").render(area, buf),
        }
    }
}

impl Screen {
    pub fn keys_iter(&self) -> impl Iterator<Item = &Key> {
        match self {
            Screen::MainScreen(x) => x.keys(),
        }
    }
}
