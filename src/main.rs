use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use rs_platform_explorer::app::App;
use rs_platform_explorer::terminal_event::{TerminalEvent, TerminalEventHandler};
use rs_platform_explorer::tui::Tui;

fn main() {
    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend).expect("cannot initialize terminal backend");
    let events = TerminalEventHandler::new();
    let mut tui = Tui::new(terminal, events);
    tui.init().expect("unable to init terminal UI");

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app).expect("unable to update terminal view");
        // Handle events.
        match tui
            .events
            .next()
            .expect("unable to get next terminal event")
        {
            TerminalEvent::Key(key_event) => app.handle_key_event(key_event),
            _ => {}
        }
    }

    // Exit the user interface.
    tui.exit()
        .expect("unable to properly close terminal session");
}
