//! Wallet screen module.

use dpp::identity::accessors::IdentityGettersV0;
use tui_realm_stdlib::Paragraph;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::{state::AppState, Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct WalletScreen {
    component: Paragraph,
}

impl WalletScreen {
    pub(crate) fn new(app_state: &AppState, message: &str) -> Self {
        let mut paragraph = Paragraph::default();
        let title = TextSpan::new("Wallet management commands");
        let wallet_loaded_text = if let Some(wallet) = app_state.loaded_wallet.as_ref() {
            TextSpan::new(format!("Wallet Loaded: {}", wallet.description()))
        } else {
            TextSpan::new(format!("No Wallet Loaded"))
        };
        let identity_loaded_text = if let Some(identity) = app_state.loaded_identity.as_ref() {
            TextSpan::new(format!(
                "Identity Loaded: Balance : {:.2} mDash, Keys: {:?}",
                identity.balance() as f64 / 100000000.0,
                identity.public_keys()
            ))
        } else if let Some((_, _, asset_lock)) = app_state
            .identity_asset_lock_private_key_in_creation
            .as_ref()
        {
            if asset_lock.is_some() {
                TextSpan::new(format!(
                    "No Identity Loaded, but transaction registered and asset lock known"
                ))
            } else {
                TextSpan::new(format!(
                    "No Identity Loaded, but transaction registered with no asset lock yet known"
                ))
            }
        } else {
            TextSpan::new(format!("No Identity Loaded"))
        };
        let message_span = TextSpan::new(message);
        paragraph = paragraph.text(
            [
                title,
                wallet_loaded_text,
                identity_loaded_text,
                message_span,
            ]
            .as_ref(),
        );
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
    pub(crate) fn new(wallet_available: bool, identity_available: bool) -> Self {
        if wallet_available {
            if identity_available {
                WalletScreenCommands {
                    component: CommandPallet::new(vec![
                        CommandPalletKey {
                            key: 'q',
                            description: "Back to Main",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'c',
                            description: "Copy Address",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'r',
                            description: "Refresh utxos and balance",
                            key_type: KeyType::Command,
                        },
                    ]),
                }
            } else {
                WalletScreenCommands {
                    component: CommandPallet::new(vec![
                        CommandPalletKey {
                            key: 'q',
                            description: "Back to Main",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'c',
                            description: "Copy Address",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'l',
                            description: "Clear asset lock transaction",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'i',
                            description: "Register identity",
                            key_type: KeyType::Command,
                        },
                        CommandPalletKey {
                            key: 'r',
                            description: "Refresh utxos and balance",
                            key_type: KeyType::Command,
                        },
                    ]),
                }
            }
        } else {
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
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::CopyWalletAddress),
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ClearAssetLockTransaction),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::RegisterIdentity),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::UpdateLoadedWalletUTXOsAndBalance),
            _ => None,
        }
    }
}
