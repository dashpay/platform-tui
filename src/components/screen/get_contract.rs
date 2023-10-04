//! Get contract screen

use tui_realm_stdlib::Textarea;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::Message,
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct GetContractScreen {
    component: Textarea,
}

impl GetContractScreen {
    pub(crate) fn new() -> Self {
        GetContractScreen {
            component: Textarea::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for GetContractScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct GetContractScreenCommands {
    component: CommandPallet,
}

impl GetContractScreenCommands {
    pub(crate) fn new() -> Self {
        GetContractScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Contract screen",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'i',
                    description: "Get by ID",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'p',
                    description: "with proof",
                    key_type: KeyType::Toggle,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for GetContractScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput),
            _ => None,
        }
    }
}
