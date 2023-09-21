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
        .constraints([Constraint::Min(10), Constraint::Max(5), Constraint::Max(2)].as_ref())
        .split(frame.size());

    // Render the screen window
    let main_widget_block = Block::default()
        .title("Platform TUI")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::new(1, 0, 0, 0));

    frame.render_widget(app.screen().clone(), main_widget_block.inner(layout[0]));
    frame.render_widget(main_widget_block, layout[0]);

    // Render keys palette
    let commands_block = Block::default()
        .title("Commands")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .padding(Padding::new(1, 0, 0, 0));

    let mut screen_keys = app.screen().keys().iter();
    let keys_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Min(1), Constraint::Min(1)].as_ref())
        .split(commands_block.inner(layout[1]));
    for row in keys_rows.iter().map(|r| {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ]
                .as_ref(),
            )
            .split(*r)
    }) {
        for col in row.iter() {
            if let Some(key) = screen_keys.next() {
                frame.render_widget(
                    Paragraph::new(format!("{}: {}", key.key, key.description)),
                    *col,
                );
            }
        }
    }
    frame.render_widget(commands_block, layout[1]);

    // Render status bar
    let status_bar_block = Block::default()
        .padding(Padding::new(2, 0, 0, 0))
        .borders(Borders::BOTTOM);
    let status_bar_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Max(20)].as_ref())
        .split(status_bar_block.inner(layout[2]));

    frame.render_widget(
        Paragraph::new(app.screen().breadcrumbs().join(" / ")),
        status_bar_layout[0],
    );

    frame.render_widget(
        Paragraph::new(format!(
            "{}",
            if app.identity_private_key.is_some() {
                ""
            } else {
                "No identity loaded"
            },
        )),
        status_bar_layout[1],
    );
    frame.render_widget(status_bar_block, layout[2]);
}
