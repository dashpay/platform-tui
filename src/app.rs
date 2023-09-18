use crate::screen::Screen;

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
