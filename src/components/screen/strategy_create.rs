//! Create strategy

use strategy_tests::{frequency::Frequency, Strategy};
use tui_realm_stdlib::{Paragraph, List};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Color, Alignment}, command::{Cmd, Direction}};

use crate::{app::{Message, state::AppState}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};
use crate::app::InputType::EditContracts;
use crate::app::strategies::StrategyDetails;

#[derive(MockComponent)]
pub(crate) struct CreateStrategyScreen {
    component: Paragraph,
}

impl CreateStrategyScreen {
    pub(crate) fn new() -> Self {
        CreateStrategyScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Strategy creation commands")].as_ref()),
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
                    description: "Back to Strategies",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Edit Contracts Field",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'o',
                    description: "Edit Operations Field",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 's',
                    description: "Edit Start Identities Field",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'i',
                    description: "Edit Identity Insertions Field",
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
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(EditContracts)),
            // Event::Keyboard(KeyEvent {
            //     code: Key::Char('o'),
            //     modifiers: KeyModifiers::NONE,
            // }) => Some(Message::ExpectingInput(EditOperations)),
            // Event::Keyboard(KeyEvent {
            //     code: Key::Char('s'),
            //     modifiers: KeyModifiers::NONE,
            // }) => Some(Message::ExpectingInput(EditStartIdentities)),
            // Event::Keyboard(KeyEvent {
            //     code: Key::Char('i'),
            //     modifiers: KeyModifiers::NONE,
            // }) => Some(Message::ExpectingInput(EditIdentityInserts)),
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct EditContractsStruct {
    component: List,
    selected_index: usize,
}

impl EditContractsStruct {
    pub(crate) fn new(app_state: &mut AppState) -> Self {
        if app_state.current_strategy.is_none() {
            app_state.current_strategy = Some("new_strategy".to_string());
            app_state.available_strategies.insert("new_strategy".to_string(), StrategyDetails {
                strategy: Strategy {
                    contracts_with_updates: vec![],
                    operations: vec![],
                    start_identities: vec![],
                    identities_inserts: Frequency {
                        times_per_block_range: Default::default(),
                        chance_per_block: None,
                    },
                    signer: None,
                    },
                description: "New default strategy".to_string(),
            });
        }

        let current_strategy = &app_state.current_strategy;
        let contracts = &app_state.known_contracts;
                
        let mut rows = TableBuilder::default();
        for (name, _) in contracts.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select a contract. Navigate with your arrow keys and press ENTER to select.", Alignment::Center)
                    .scroll(true)
                    .highlighted_color(Color::LightYellow)
                    .highlighted_str("> ")
                    .rewind(true)
                    .step(1)
                    .rows(rows.build())
                    .selected_line(0),
                selected_index: 0,
        }
    }
}

impl Component<Message, NoUserEvent> for EditContractsStruct {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                self.selected_index = self.selected_index + 1;
                self.perform(Cmd::Move(Direction::Down));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent { 
                code: Key::Up, .. 
            }) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }            
                self.perform(Cmd::Move(Direction::Up));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                Some(Message::AddStrategyContract(self.selected_index))
            }
            _ => None,
        }
    }
}
