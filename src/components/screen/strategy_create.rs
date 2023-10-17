//! Create strategy

use tui_realm_stdlib::List;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}};

use crate::{app::Message, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct CreateStrategyScreen {
    component: List,
}

impl CreateStrategyScreen {
    pub(crate) fn new() -> Self {
        CreateStrategyScreen {
            component: List::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for CreateStrategyScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct CreateStrategyScreenCommands {
    component: CommandPallet,
}

impl CreateStrategyScreenCommands {
    pub(crate) fn new() -> Self {
        CreateStrategyScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for CreateStrategyScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                                code: Key::Char('q'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::PrevScreen),
            _ => None,
        }
    }
}