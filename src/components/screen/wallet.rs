//! Wallet screen module.

use tui_realm_stdlib::Paragraph;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::app::state::AppState;
use crate::{
    app::{Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct WalletScreen {
    component: Paragraph,
}

impl WalletScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let mut paragraph = Paragraph::default();
        let title = TextSpan::new("Wallet management commands");
        let loaded_text = if let Some(wallet) = app_state.loaded_wallet.as_ref() {
            TextSpan::new(format!("Wallet Loaded: {}", wallet.description()))
        } else {
            TextSpan::new(format!("No Wallet Loaded"))
        };
        paragraph = paragraph.text([title, loaded_text].as_ref());
        WalletScreen {
            component: paragraph,
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
                CommandPalletKey {
                    key: 'f',
                    description: "Fetch utxos and balance",
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
            Event::Keyboard(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::UpdateLoadedWalletUTXOsAndBalance),
            _ => None,
        }
    }
}
