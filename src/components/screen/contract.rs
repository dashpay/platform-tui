//! Contract screen module.

use tui_realm_stdlib::{Paragraph, Table};
use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::{Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct ContractScreen {
    component: Paragraph,
}

impl ContractScreen {
    pub(crate) fn new() -> Self {
        ContractScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Contract management commands")].as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for ContractScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct ContractScreenCommands {
    component: CommandPallet,
}

impl ContractScreenCommands {
    pub(crate) fn new() -> Self {
        ContractScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'g',
                    description: "Get Contract",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for ContractScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::GetContract)),
            _ => None,
        }
    }
}
