//! Strategy Contracts screen

use std::collections::BTreeMap;

use dpp::{prelude::DataContract, data_contract::{document_type::{DocumentType, random_document::{DocumentFieldFillType, DocumentFieldFillSize}}, created_data_contract::CreatedDataContract, accessors::v0::DataContractV0Getters}};
use strategy_tests::operations::DocumentAction;
use tui_realm_stdlib::{Paragraph, List};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Alignment}, command::{Cmd, Direction}};

use crate::{app::{Message, state::AppState, strategies::default_strategy_details}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};
use crate::app::InputType::SelectOperationType;

#[derive(MockComponent)]
pub(crate) struct StrategyOperationsScreen {
    component: Paragraph,
}

impl StrategyOperationsScreen {
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

        StrategyOperationsScreen {
            component: Paragraph::default().text(combined_spans.as_ref())
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyOperationsScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategyOperationsScreenCommands {
    component: CommandPallet,
}

impl StrategyOperationsScreenCommands {
    pub(crate) fn new() -> Self {
        StrategyOperationsScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Go Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'a',
                    description: "Add operation",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "Remove last operation",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyOperationsScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(SelectOperationType)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::RemoveOperation),
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct SelectOperationTypeStruct {
    component: List,
    selected_index: usize,
}

impl SelectOperationTypeStruct {
    pub(crate) fn new(app_state: &mut AppState) -> Self {
        if app_state.current_strategy.is_none() {
            app_state.current_strategy = Some("new_strategy".to_string());
            app_state.available_strategies.insert("new_strategy".to_string(), default_strategy_details());
        }

        let op_types: Vec<String> = vec![
            "Document".to_string(), 
            // "IdentityTopUp".to_string(), 
            // "IdentityUpdate".to_string(), 
            // "IdentityWithdrawal".to_string(),
            // "ContractCreate".to_string(),
            // "ContractUpdate".to_string(),
            // "IdentityTransfer".to_string(),
            ];
                
        let mut rows = TableBuilder::default();
        for name in op_types.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select an operation type. Navigate with your arrow keys and press ENTER to select. Press 'q' to go back.", Alignment::Center)
                    .scroll(true)
                    .highlighted_str("> ")
                    .rewind(true)
                    .step(1)
                    .rows(rows.build())
                    .selected_line(0),
            selected_index: 0,
        }
    }
}

impl Component<Message, NoUserEvent> for SelectOperationTypeStruct {
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
                Some(Message::SelectOperationType(self.selected_index))
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

enum DocumentSelectionState {
    SelectingContract,
    SelectingDocumentType { contract: DataContract },
    SelectingAction { contract: DataContract, doc_type: DocumentType },
    SelectingTimesPerBlockRange { contract: DataContract, doc_type: DocumentType, action: DocumentAction },
    // SelectingChancePerBlock,
}

#[derive(MockComponent)]
pub(crate) struct DocumentStruct {
    component: List,
    selected_index: usize,
    selection_state: DocumentSelectionState,
    known_contracts: BTreeMap<String, CreatedDataContract>,
    available_document_types: Option<BTreeMap<String, DocumentType>>,
}

impl DocumentStruct {
    fn update_component_for_document_types(&mut self) {
        self.selected_index = 0;
        let document_types = self.available_document_types.as_ref().unwrap();
        let mut rows = TableBuilder::default();
        for doc_type in document_types {
            rows.add_col(TextSpan::from(doc_type.0));
            rows.add_row();
        }
        self.component = List::default()
            .title("Select a document type. Navigate with your arrow keys and press ENTER to select.", Alignment::Center)
            .scroll(true)
            .highlighted_str("> ")
            .rewind(true)
            .step(1)
            .rows(rows.build())
            .selected_line(0);
    }

    fn update_component_for_actions(&mut self) {
        self.selected_index = 0;
        let actions = vec!["InsertRandom", "Delete", "Replace"];
        let mut rows = TableBuilder::default();
        for action in actions {
            rows.add_col(TextSpan::from(action));
            rows.add_row();
        }
        self.component = List::default()
            .title("Select an action. Navigate with your arrow keys and press ENTER to select.", Alignment::Center)
            .scroll(true)
            .highlighted_str("> ")
            .rewind(true)
            .step(1)
            .rows(rows.build())
            .selected_line(0);
    }

    fn update_component_for_frequency(&mut self) {
        self.selected_index = 0;
        let ranges = vec!["1", "2", "5", "10", "20", "40"];
        let mut rows = TableBuilder::default();
        for range in ranges {
            rows.add_col(TextSpan::from(range));
            rows.add_row();
        }
        self.component = List::default()
            .title("Select the maximum times per block for the action to occur", Alignment::Center)
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
            app_state.available_strategies.insert("new_strategy".to_string(), default_strategy_details());
        }

        let contracts = &app_state.known_contracts;
        let mut rows = TableBuilder::default();
        for (name, _) in contracts.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                .title("Select a contract. Navigate with your arrow keys and press ENTER to select. Press 'q' to go back.", Alignment::Center)
                .scroll(true)
                .highlighted_str("> ")
                .rewind(true)
                .step(1)
                .rows(rows.build())
                .selected_line(0),
            selected_index: 0,
            selection_state: DocumentSelectionState::SelectingContract,
            known_contracts: contracts.clone(),
            available_document_types: None,
        }
    }
}

impl Component<Message, NoUserEvent> for DocumentStruct {
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
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                match &mut self.selection_state {
                    DocumentSelectionState::SelectingContract => {
                        let contract_clone = self.known_contracts.values().nth(self.selected_index).unwrap().clone();
                        let selected_contract = contract_clone.data_contract();
                        let document_types = selected_contract.document_types();
                                                
                        self.selection_state = DocumentSelectionState::SelectingDocumentType { contract: selected_contract.clone() };
                        self.available_document_types = Some(document_types.clone());
                        self.update_component_for_document_types();
                        
                        Some(Message::Redraw)
                    },
                    DocumentSelectionState::SelectingDocumentType { contract } => {
                        let selected_document_type = self.available_document_types.clone().unwrap().values().nth(self.selected_index).unwrap().clone();
                                
                        self.selection_state = DocumentSelectionState::SelectingAction { 
                            contract: contract.clone(),
                            doc_type: selected_document_type.clone(),
                        };
                        self.update_component_for_actions();
                        
                        Some(Message::Redraw)
                    },
                    DocumentSelectionState::SelectingAction { contract, doc_type } => {
                        let action = match self.selected_index {
                            0 => DocumentAction::DocumentActionInsertRandom(DocumentFieldFillType::FillIfNotRequired, DocumentFieldFillSize::AnyDocumentFillSize),
                            1 => DocumentAction::DocumentActionDelete,
                            2 => DocumentAction::DocumentActionReplace,
                            _ => panic!("Invalid action index"),
                        };

                        self.selection_state = DocumentSelectionState::SelectingTimesPerBlockRange { 
                            contract: contract.clone(),
                            doc_type: doc_type.clone(),
                            action: action.clone(),
                        };

                        self.update_component_for_frequency();

                        Some(Message::Redraw)
                    },
                    DocumentSelectionState::SelectingTimesPerBlockRange { contract, doc_type, action } => {
                        let tpbr = match self.selected_index {
                            0 => 1,
                            1 => 2,
                            2 => 5,
                            3 => 10,
                            4 => 20,
                            5 => 40,
                            _ => panic!("Invalid tpbr index"),
                        };
                        Some(Message::DocumentOp(
                            contract.clone(), 
                            doc_type.clone(),
                            action.clone(),
                            tpbr, 
                        ))
                    },
                    // DocumentSelectionState::SelectingChancePerBlock => {

                    // },
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
