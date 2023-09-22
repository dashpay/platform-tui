//! Main screen module, also known as a welcome screen.

use tui_realm_stdlib::{Paragraph, Table};
use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::app::{Message, Screen};

#[derive(MockComponent)]
pub(crate) struct MainScreen {
    component: Paragraph,
}

impl MainScreen {
    pub(crate) fn new() -> Self {
        MainScreen {
            component: Paragraph::default().text(
                [
                    TextSpan::new("Welcome to Platform TUI!"),
                    TextSpan::new("Use keys listed in the section below to switch screens and/or toggle flags."),
                    TextSpan::new("Some of them require signature and are disabled until an identity key is loaded.")
                ]
                .as_ref(),
            ),
        }
    }
}

impl Component<Message, NoUserEvent> for MainScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct MainScreenCommands {
    component: Table,
}

impl MainScreenCommands {
    pub(crate) fn new() -> Self {
        MainScreenCommands {
            component: Table::default().table(vec![vec![
                TextSpan::new("q - Quit"),
                TextSpan::new("i - Identities"),
            ]]),
        }
    }
}

impl Component<Message, NoUserEvent> for MainScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ChangeScreen(Screen::Identity)),
            _ => None,
        }
    }
}
