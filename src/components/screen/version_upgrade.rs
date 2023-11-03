//! Version upgrade related screens

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
pub(crate) struct VersionUpgradeCommands {
    component: CommandPallet,
}

impl VersionUpgradeCommands {
    pub(crate) fn new() -> Self {
        VersionUpgradeCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'u',
                    description: "Version upgrade state",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'v',
                    description: "Version upgrade vote status",
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

impl Component<Message, NoUserEvent> for VersionUpgradeCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('u'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::FetchVersionUpgradeState),
            _ => None,
        }
    }
}
