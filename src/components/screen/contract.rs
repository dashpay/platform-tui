//! Contract screen module.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::string_encoding::Encoding;
use std::vec;
use tui_realm_stdlib::{Paragraph, Textarea};
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
use crate::components::screen::shared::Info;

#[derive(MockComponent)]
pub(crate) struct ContractScreen {
    component: Info<true, false>,
}

impl ContractScreen {
    pub(crate) fn new(state: &AppState) -> Self {
        let text_spans = if state.known_contracts.is_empty() {
            vec![TextSpan::new("No known contracts, fetch some!")]
        } else {
            state
                .known_contracts
                .iter()
                .map(|(name, contract)| {
                    TextSpan::new(format!(
                        "{} : {} ({} Types)",
                        name,
                        contract.id_ref().to_string(Encoding::Base58),
                        contract.document_types().len()
                    ))
                })
                .collect::<Vec<_>>()
        };

        ContractScreen {
            component: Info::new_scrollable_text_rows(text_spans.as_slice())
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
                    key: 'u',
                    description: "Fetch User Contract",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Fetch System Contract",
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
                code: Key::Char('u'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::FetchUserContract(None))),
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::FetchSystemContract(None))),
            _ => None,
        }
    }
}
