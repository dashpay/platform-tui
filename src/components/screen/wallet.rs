//! Wallet screen module.

use tui_realm_stdlib::Paragraph;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::{Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct WalletScreen {
    component: Paragraph,
}

impl WalletScreen {
    pub(crate) fn new() -> Self {
        WalletScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Wallet management commands")].as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for WalletScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct WalletScreenCommands {
    component: CommandPallet,
}

impl WalletScreenCommands {
    pub(crate) fn new() -> Self {
        WalletScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'a',
                    description: "Add wallet",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for WalletScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                                code: Key::Char('q'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                                code: Key::Char('a'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::NextScreen(Screen::AddWallet)),
            _ => None,
        }
    }
}
