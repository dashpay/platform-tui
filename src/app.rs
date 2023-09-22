//! Application logic module, includes model and screen ids.

use std::time::Duration;

use tuirealm::{terminal::TerminalBridge, Application, EventListenerCfg, NoUserEvent, Update};

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
    Screen(Screen),
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
    /// Used to draw to terminal
    pub terminal: TerminalBridge,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            app: Self::init_app(),
            quit: false,
            redraw: true,
            terminal: TerminalBridge::new().expect("Cannot initialize terminal"),
        }
    }
}

impl Model {
    fn init_app() -> Application<ComponentId, Message, NoUserEvent> {
        // Setup application
        // NOTE: NoUserEvent is a shorthand to tell tui-realm we're not going to use any custom user event
        // NOTE: the event listener is configured to use the default crossterm input listener and to raise a Tick event each second
        // which we will use to update the clock

        let mut app = Application::init(
            EventListenerCfg::default()
                .default_input_listener(Duration::from_millis(20))
                .poll_timeout(Duration::from_millis(10))
                .tick_interval(Duration::from_secs(1)),
        );
        // Mount components
        // assert!(app
        //     .mount(
        //         Id::Label,
        //         Box::new(
        //             Label::default()
        //                 .text("Waiting for a Msg...")
        //                 .alignment(Alignment::Left)
        //                 .background(Color::Reset)
        //                 .foreground(Color::LightYellow)
        //                 .modifiers(TextModifiers::BOLD),
        //         ),
        //         Vec::default(),
        //     )
        //     .is_ok());
        // Mount clock, subscribe to tick
        // assert!(app
        //     .mount(
        //         Id::Clock,
        //         Box::new(
        //             Clock::new(SystemTime::now())
        //                 .alignment(Alignment::Center)
        //                 .background(Color::Reset)
        //                 .foreground(Color::Cyan)
        //                 .modifiers(TextModifiers::BOLD)
        //         ),
        //         vec![Sub::new(SubEventClause::Tick, SubClause::Always)]
        //     )
        //     .is_ok());
        app
    }

    pub fn view(&mut self) {
        // assert!(self
        //     .terminal
        //     .raw_mut()
        //     .draw(|f| {
        //         let chunks = Layout::default()
        //             .direction(Direction::Vertical)
        //             .margin(1)
        //             .constraints(
        //                 [
        //                     Constraint::Length(3), // Clock
        //                     Constraint::Length(3), // Letter Counter
        //                     Constraint::Length(3), // Digit Counter
        //                     Constraint::Length(1), // Label
        //                 ]
        //                 .as_ref(),
        //             )
        //             .split(f.size());
        //         self.app.view(&Id::Clock, f, chunks[0]);
        //         self.app.view(&Id::LetterCounter, f, chunks[1]);
        //         self.app.view(&Id::DigitCounter, f, chunks[2]);
        //         self.app.view(&Id::Label, f, chunks[3]);
        //     })
        //     .is_ok());
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
