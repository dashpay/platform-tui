use std::io;

use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use rs_platform_explorer::app::{App, AppResult};
use rs_platform_explorer::event::{Event, EventHandler};
use rs_platform_explorer::handler::handle_key_events;
use rs_platform_explorer::tui::Tui;

fn main() -> AppResult<()> {
    // Create an application.
    let mut app = App::new();

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new();
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            _ => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
