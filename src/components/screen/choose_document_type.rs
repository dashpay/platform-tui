//! DocumentType screen module.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::DataContract;
use std::vec;
use tui_realm_stdlib::List;
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

#[derive(MockComponent)]
pub(crate) struct ChooseDocumentTypeScreen {
    component: List,
}

impl ChooseDocumentTypeScreen {
    pub(crate) fn new(data_contract: DataContract) -> Self {
        let text_spans = if data_contract.document_types().is_empty() {
            vec![vec![TextSpan::new(
                "Contract has no document types, very weird!",
            )]]
        } else {
            data_contract
                .document_types()
                .iter()
                .map(|(name, document_type)| {
                    vec![TextSpan::new(format!(
                        "{} : {}",
                        name,
                        document_type
                            .properties()
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("|")
                    ))]
                })
                .collect::<Vec<_>>()
        };

        let mut component = List::default()
            .rows(text_spans)
            .highlighted_color(Color::LightYellow);
        component.attr(Attribute::Scroll, AttrValue::Flag(true));
        component.attr(Attribute::Focus, AttrValue::Flag(true));

        ChooseDocumentTypeScreen { component }
    }
}

impl Component<Message, NoUserEvent> for ChooseDocumentTypeScreen {
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
            }) => Some(Message::SelectDocumentType(
                self.component.state().unwrap_one().unwrap_usize(),
            )),
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct ChooseDocumentTypeScreenCommands {
    component: CommandPallet,
}

impl ChooseDocumentTypeScreenCommands {
    pub(crate) fn new() -> Self {
        ChooseDocumentTypeScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Main",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'v',
                    description: "View More",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for ChooseDocumentTypeScreenCommands {
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
