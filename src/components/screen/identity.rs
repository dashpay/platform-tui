//! Identity screen module.

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
    component: CommandPallet,
}

impl IdentityScreenCommands {
    pub(crate) fn new() -> Self {
        IdentityScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'g',
                    description: "Get Identity",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::GetIdentity)),
            _ => None,
        }
    }
}
