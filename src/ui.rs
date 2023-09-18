use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};

use crate::app::App;

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    // App layout: screen window, screen keys and status bar
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Max(10), Constraint::Max(2)].as_ref())
        .split(frame.size());

    // Render the screen window
    let main_widget_block = Block::default()
        .title("Platform Explorer")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::new(1, 0, 0, 0));

    frame.render_widget(app.screen.clone(), main_widget_block.inner(layout[0]));
    frame.render_widget(main_widget_block, layout[0]);

    frame.render_widget(
        Paragraph::new(format!("Test",)).block(
            Block::default()
                .title("Commands")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .padding(Padding::new(1, 0, 0, 0)),
        ),
        layout[1],
    );

    frame.render_widget(
        Paragraph::new("Modeline").block(
            Block::default().padding(Padding::new(1, 0, 0, 0)).borders(
                Borders::from_bits_truncate(
                    Borders::LEFT.bits() | Borders::RIGHT.bits() | Borders::BOTTOM.bits(),
                ),
            ),
        ),
        layout[2],
    );
}
