//! Identity screen module.

use tui_realm_stdlib::{Paragraph, Table};
use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::app::{Message, Screen};

#[derive(MockComponent)]
pub(crate) struct IdentityScreen {
    component: Paragraph,
}

impl IdentityScreen {
    pub(crate) fn new() -> Self {
        IdentityScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Identity management commands")].as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct IdentityScreenCommands {
    component: Table,
}

impl IdentityScreenCommands {
    pub(crate) fn new() -> Self {
        IdentityScreenCommands {
            component: Table::default().table(vec![vec![TextSpan::new("q - Back to Main")]]),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ChangeScreen(Screen::Main)),
            _ => None,
        }
    }
}
