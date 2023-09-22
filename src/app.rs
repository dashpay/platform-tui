//! Application logic module, includes model and screen ids.

use std::time::Duration;

use tui_realm_stdlib::Label;
use tuirealm::{
    terminal::TerminalBridge,
    tui::prelude::{Constraint, Direction, Layout},
    Application, ApplicationError, EventListenerCfg, NoUserEvent, Update,
};

use crate::components::{MainScreen, Status};

/// Screen identifiers
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(super) enum Screen {
    Main,
    Identity,
}

/// Component identifiers, required to triggers screen switch which involves mounting and
/// unmounting.
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(super) enum ComponentId {
    CommandPallet,
    Screen,
    Status,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Main
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum Message {
    AppClose,
    ChangeScreen(Screen),
}

pub(super) struct Model {
    /// Application
    pub app: Application<ComponentId, Message, NoUserEvent>,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Current screen
    pub current_screen: Screen,
    /// Used to draw to terminal
    pub terminal: TerminalBridge,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            app: Self::init_app().expect("Unable to init the application"),
            quit: false,
            redraw: true,
            current_screen: Screen::Main,
            terminal: TerminalBridge::new().expect("Cannot initialize terminal"),
        }
    }
}

impl Model {
    fn init_app() -> Result<Application<ComponentId, Message, NoUserEvent>, ApplicationError> {
        let mut app = Application::init(
            EventListenerCfg::default()
                .default_input_listener(Duration::from_millis(20))
                .poll_timeout(Duration::from_millis(10))
                .tick_interval(Duration::from_secs(1)),
        );

        // Mount components
        app.mount(ComponentId::Status, Box::new(Status::new()), Vec::new())?;
        app.mount(ComponentId::Screen, Box::new(MainScreen::new()), Vec::new())?;

        // Setting focus on the screen so it will react to events
        app.active(&ComponentId::Screen)?;

        Ok(app)
    }

    pub fn view(&mut self) {
        self.terminal
            .raw_mut()
            .draw(|f| {
                // App layout: screen window, screen keys and status bar
                let outer_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [Constraint::Min(10), Constraint::Max(5), Constraint::Max(2)].as_ref(),
                    )
                    .split(f.size());

                // Status line layout
                let status_bar_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(20), Constraint::Max(20)].as_ref())
                    .split(outer_layout[2]);

                self.app.view(&ComponentId::Status, f, status_bar_layout[1]);
            })
            .expect("unable to render the application");
    }
}

impl Update<Message> for Model {
    fn update(&mut self, message: Option<Message>) -> Option<Message> {
        if let Some(message) = message {
            // Set redraw
            self.redraw = true;
            // Match message
            match message {
                Message::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                _ => todo!(),
            }
        } else {
            None
        }
    }
}
