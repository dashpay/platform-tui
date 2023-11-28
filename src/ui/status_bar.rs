//! Status bar component definitions.

use std::fmt::{self, Display};

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
    breadcrumbs: Vec<&'static str>,
    blocked: bool,
    identity_loaded_balance: IdentityBalanceStatus,
}

enum IdentityBalanceStatus {
    NoIdentity,
    Balance(u64),
    RefreshError,
}

impl Default for IdentityBalanceStatus {
    fn default() -> Self {
        IdentityBalanceStatus::NoIdentity
    }
}

impl Display for IdentityBalanceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityBalanceStatus::NoIdentity => write!(f, "No identity"),
            IdentityBalanceStatus::Balance(balance) => write!(f, "Platform balance: {}", balance),
            IdentityBalanceStatus::RefreshError => write!(f, "Balance refresh error"),
        }
    }
}

impl StatusBarState {
    pub(crate) fn with_balance(balance: u64) -> Self {
        StatusBarState {
            identity_loaded_balance: IdentityBalanceStatus::Balance(balance),
            ..Default::default()
        }
    }

    pub(crate) fn update_balance(&mut self, balance: u64) {
        self.identity_loaded_balance = IdentityBalanceStatus::Balance(balance);
    }

    pub(crate) fn set_balance_error(&mut self) {
        self.identity_loaded_balance = IdentityBalanceStatus::RefreshError;
    }

    pub(crate) fn block(&mut self) {
        self.blocked = true;
    }

    pub(crate) fn unblock(&mut self) {
        self.blocked = false;
    }

    pub(crate) fn add_child(&mut self, name: &'static str) {
        self.breadcrumbs.push(name);
    }

    pub(crate) fn to_parent(&mut self) {
        self.breadcrumbs.pop();
    }

    pub(crate) fn view(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new().borders(BorderSides::ALL);

        let layout = Layout::default()
            .horizontal_margin(1)
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Max(20)].as_ref())
            .split(block.inner(area));

        let breadcrumbs_str = self.breadcrumbs.join(" / ");

        if self.blocked {
            Label::default()
                .text("Executing a task, please wait")
                .modifiers(Modifier::RAPID_BLINK) // TODO: doesn't work lol
        } else {
            Label::default().text(&breadcrumbs_str)
        }
        .view(frame, layout[0]);

        Label::default()
            .text(&self.identity_loaded_balance.to_string())
            .view(frame, layout[1]);

        frame.render_widget(block, area);
    }
}
