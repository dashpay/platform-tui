//! DocumentType screen module.

use std::vec;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::prelude::DataContract;
use tui_realm_stdlib::{List};
use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Color, TextSpan},
    AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::{Message, Screen},
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};
use crate::components::screen::shared::Info;

#[derive(MockComponent)]
pub(crate) struct DocumentTypeScreen {
    component: Info<true, false>,
    data_contract: DataContract,
    document_type: DocumentType,
}

impl DocumentTypeScreen {
    pub(crate) fn new(data_contract: DataContract, document_type: DocumentType) -> Self {
        let component = Info::new_scrollable(
            toml::to_string_pretty(&document_type.properties())
                .as_deref()
                .unwrap_or("cannot serialize as TOML"),
        );

        DocumentTypeScreen { component, data_contract, document_type }
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
            // Event::Keyboard(KeyEvent {
            //                     code: Key::Char('v'),
            //                     modifiers: KeyModifiers::NONE,
            //                 }) => Some(Message::NextScreen(Screen::FetchUserDocumentType(None))),
            // Event::Keyboard(KeyEvent {
            //                     code: Key::Char('c'),
            //                     modifiers: KeyModifiers::NONE,
            //                 }) => Some(Message::NextScreen(Screen::FetchSystemDocumentType(None))),
            _ => None,
        }
    }
}
