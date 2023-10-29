//! Strategy Contracts screen

use std::collections::BTreeMap;

use dpp::data_contract::created_data_contract::CreatedDataContract;
use tui_realm_stdlib::{Paragraph, List};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Alignment}, command::{Cmd, Direction}};

use crate::{app::{Message, state::AppState, strategies::default_strategy_details}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};
use crate::app::InputType::AddContract;

#[derive(MockComponent)]
pub(crate) struct StrategyContractsScreen {
    component: Paragraph,
}

impl StrategyContractsScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {

        let mut combined_spans = Vec::new();
        if let Some(strategy_key) = &app_state.current_strategy {
            // Append the current strategy name in bold to combined_spans
            combined_spans.push(TextSpan::new(&format!("{}:", strategy_key)).bold());
        
            if let Some(strategy) = app_state.available_strategies.get(strategy_key) {
                for (key, value) in &strategy.description {
                    combined_spans.push(TextSpan::new(&format!("  {}:", key)).bold());
                    combined_spans.push(TextSpan::new(&format!("    {}",value)));
                }
            } else {
                // Handle the case where the strategy_key doesn't exist in available_strategies
                combined_spans.push(TextSpan::new("Error: current strategy not found in available strategies."));
            }
        } else {
            // Handle the case where app_state.current_strategy is None
            combined_spans.push(TextSpan::new("No strategy loaded.").bold());
        }

        StrategyContractsScreen {
            component: Paragraph::default().text(combined_spans.as_ref())
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyContractsScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategyContractsScreenCommands {
    component: CommandPallet,
}

impl StrategyContractsScreenCommands {
    pub(crate) fn new() -> Self {
        StrategyContractsScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'a',
                    description: "Add",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "Remove last",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyContractsScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(AddContract)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::RemoveContract),
            _ => None,
        }
    }
}

pub enum StrategySelectionState {
    SelectFirst,
    UpdatesOption { contracts: Vec<String> },
    SelectSubsequent { contracts: Vec<String> },
}

#[derive(MockComponent)]
pub(crate) struct AddContractStruct {
    component: List,
    selected_index: usize,
    selection_state: StrategySelectionState,
    known_contracts: BTreeMap<String, CreatedDataContract>,
}

impl AddContractStruct {
    fn update_component_for_contract_update_option(&mut self) {
        self.selected_index = 0;
        let options = vec!["yes", "no"];
        let mut rows = TableBuilder::default();
        for option in options {
            rows.add_col(TextSpan::from(option));
            rows.add_row();
        }
        self.component = List::default()
            .title("Would you like to add a contract update?", Alignment::Center)
            .scroll(true)
            .highlighted_str("> ")
            .rewind(true)
            .step(1)
            .rows(rows.build())
            .selected_line(0);
    }

    fn update_component_for_contract_update(&mut self) {
        self.selected_index = 0;
        let options = self.known_contracts.keys();
        let mut rows = TableBuilder::default();
        for option in options {
            rows.add_col(TextSpan::from(option));
            rows.add_row();
        }
        self.component = List::default()
            .title("Select a contract update or press 'q' to go back", Alignment::Center)
            .scroll(true)
            .highlighted_str("> ")
            .rewind(true)
            .step(1)
            .rows(rows.build())
            .selected_line(0);
    }

    pub(crate) fn new(app_state: &mut AppState) -> Self {
        if app_state.current_strategy.is_none() {
            app_state.current_strategy = Some("new_strategy".to_string());
            app_state.available_strategies.insert("new_strategy".to_string(), default_strategy_details(),
            );
        }

        let contracts = &app_state.known_contracts;
                
        let mut rows = TableBuilder::default();
        for (name, _) in contracts.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select a contract. Press 'q' to go back.", Alignment::Center)
                    .scroll(true)
                    .highlighted_str("> ")
                    .rewind(true)
                    .step(1)
                    .rows(rows.build())
                    .selected_line(0),
            selected_index: 0,
            selection_state: StrategySelectionState::SelectFirst,
            known_contracts: app_state.known_contracts.clone(),
        }
    }
}

impl Component<Message, NoUserEvent> for AddContractStruct {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                let max_index = self.component.states.list_len-2;
                if self.selected_index < max_index {
                    self.selected_index = self.selected_index + 1;
                    self.perform(Cmd::Move(Direction::Down));
                }
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent { 
                code: Key::Up, .. 
            }) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.perform(Cmd::Move(Direction::Up));
                }            
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                match &mut self.selection_state {
                    StrategySelectionState::SelectFirst => {
                        let mut contracts_with_updates = Vec::new();
                        let (name, _) = self.known_contracts.iter().nth(self.selected_index).map(|(k, v)| (k.clone(), v.clone())).unwrap();
                        contracts_with_updates.push(name);

                        self.selection_state = StrategySelectionState::UpdatesOption { contracts: contracts_with_updates };
                        self.update_component_for_contract_update_option();
                        
                        Some(Message::Redraw)                        
                    },
                    StrategySelectionState::UpdatesOption { contracts } => {
                        // would you like to add contract updates?
                        match self.selected_index {
                            // yes
                            0 => {
                                self.selection_state = StrategySelectionState::SelectSubsequent { contracts: contracts.clone() };
                                self.update_component_for_contract_update();
                                
                                Some(Message::Redraw)                                
                            },
                            // no
                            1 => {
                                Some(Message::AddStrategyContract(contracts.clone()))
                            },
                            _ => {
                                panic!("invalid index in StrategySelectionState::UpdatesOption")
                            }
                        }
                    },
                    StrategySelectionState::SelectSubsequent { contracts } => {
                        let (name, _) = self.known_contracts.iter().nth(self.selected_index).map(|(k, v)| (k.clone(), v.clone())).unwrap();
                        contracts.push(name);

                        self.selection_state = StrategySelectionState::UpdatesOption { contracts: contracts.clone() };
                        self.update_component_for_contract_update_option();
                        
                        Some(Message::Redraw)                        
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'), ..
            }) => {
                Some(Message::ReloadScreen)
            }
            _ => None,
        }
    }
}