//! Status bar component definitions.

use tui_realm_stdlib::Label;
use tuirealm::{
    props::BorderSides,
    tui::{
        prelude::{Constraint, Direction, Layout, Modifier, Rect},
        widgets::Block,
    },
    Frame, MockComponent,
};

#[derive(Default)]
pub(crate) struct StatusBarState {
    pub breadcrumbs: Vec<&'static str>,
    pub blocked: bool,
    pub identity_private_keys_loaded: bool,
}

impl StatusBarState {
    pub(crate) fn add_child(&mut self, name: &'static str) {
        self.breadcrumbs.push(name);
    }

    pub(crate) fn to_parent(&mut self) {
        self.breadcrumbs.pop();
    }
}

pub(crate) fn view(frame: &mut Frame, area: Rect, state: &StatusBarState) {
    let block = Block::new().borders(BorderSides::ALL);

    let layout = Layout::default()
        .horizontal_margin(1)
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Max(20)].as_ref())
        .split(block.inner(area));

    let breadcrumbs_str = state.breadcrumbs.join(" / ");
    let identity_private_keys_loaded_str = if state.identity_private_keys_loaded {
        "Identity loaded"
    } else {
        "NO Identity"
    };

    if state.blocked {
        Label::default()
            .text("Executing a task, please wait")
            .modifiers(Modifier::RAPID_BLINK) // TODO: doesn't work lol
    } else {
        Label::default().text(&breadcrumbs_str)
    }
    .view(frame, layout[0]);

    Label::default()
        .text(identity_private_keys_loaded_str)
        .view(frame, layout[1]);

    frame.render_widget(block, area);
}
