use std::error;

use crate::screen::{self, Screen};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Current screen
    pub screen: Screen,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            screen: Screen::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
