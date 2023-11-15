//! Contract screen module.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::string_encoding::Encoding;
use std::vec;
use tui_realm_stdlib::{List, Paragraph, Textarea};
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
pub(crate) struct ContractScreen {
    component: List,
}

impl ContractScreen {
    pub(crate) fn new(state: &AppState) -> Self {
        let text_spans = if state.known_contracts.is_empty() {
            vec![vec![TextSpan::new("No known contracts, fetch some!")]]
        } else {
            state
                .known_contracts
                .iter()
                .map(|(name, contract)| {
                    vec![TextSpan::new(format!(
                        "{} : {} ({} Types)",
                        name,
                        contract.id_ref().to_string(Encoding::Base58),
                        contract.document_types().len()
                    ))]
                })
                .collect::<Vec<_>>()
        };

        let mut component = List::default()
            .rows(text_spans)
            .highlighted_color(Color::LightYellow);
        component.attr(Attribute::Scroll, AttrValue::Flag(true));
        component.attr(Attribute::Focus, AttrValue::Flag(true));

        ContractScreen { component }
    }
}

impl Component<Message, NoUserEvent> for ContractScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(
                KeyEvent {
                    code: Key::Down,
                    modifiers: KeyModifiers::NONE,
                }
                | KeyEvent {
                    code: Key::Char('n'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Move(Direction::Down));
                Some(Message::Redraw)
            }
            Event::Keyboard(
                KeyEvent {
                    code: Key::Up,
                    modifiers: KeyModifiers::NONE,
                }
                | KeyEvent {
                    code: Key::Char('p'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Move(Direction::Up));
                Some(Message::Redraw)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::SelectContract(
                self.component.state().unwrap_one().unwrap_usize(),
            )),
            _ => None,
        }
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
