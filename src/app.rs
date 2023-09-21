use crossterm::event::KeyEvent;

use crate::screen::Screen;

/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Current screen
    pub screen: Option<Screen>,
    /// Client identity,
    pub identity_private_key: Option<()>, // TODO
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            screen: Some(Screen::default()),
            identity_private_key: None,
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

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        let screen = self.screen.take().unwrap_or_default();
        self.screen = Some(screen.handle_key_event(self, key_event));
    }

    pub fn screen(&self) -> &Screen {
        self.screen.as_ref().expect("must be in place")
    }
}
