//! DocumentType screen module.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::prelude::DataContract;
use std::vec;
use tui_realm_stdlib::List;
use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Color, TextSpan},
    AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent,
};

use crate::app::state::AppState;
use crate::components::screen::shared::Info;
use crate::{
    app::{Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};

#[derive(MockComponent)]
pub(crate) struct DocumentTypeScreen {
    component: Info<true, false>,
}

impl DocumentTypeScreen {
    pub(crate) fn new(data_contract: &DataContract, document_type: &DocumentType) -> Self {
        let component = Info::new_scrollable(
            toml::to_string_pretty(&document_type.properties())
                .as_deref()
                .unwrap_or("cannot serialize as TOML"),
        );

        DocumentTypeScreen { component }
    }
}

impl Component<Message, NoUserEvent> for DocumentTypeScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct DocumentTypeScreenCommands {
    component: CommandPallet,
}

impl DocumentTypeScreenCommands {
    pub(crate) fn new() -> Self {
        DocumentTypeScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "broadcast Random document",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'm',
                    description: "query mine",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for DocumentTypeScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::BroadcastRandomDocument),
            _ => None,
        }
    }
}
