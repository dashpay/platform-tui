//! Create strategy


use std::collections::BTreeMap;

use strategy_tests::operations::DocumentAction;
use tui_realm_stdlib::{Paragraph, List, Input};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Alignment}, command::{Cmd, Direction, CmdResult}, State, StateValue};

use crate::{app::{Message, state::AppState, strategies::default_strategy_details}, mock_components::{CommandPallet, CommandPalletKey, KeyType, key_event_to_cmd}};
use crate::app::InputType::{EditContracts, RenameStrategy, LoadStrategy, SelectOperationType};
use dpp::{data_contract::{created_data_contract::CreatedDataContract, document_type::{DocumentType, random_document::{DocumentFieldFillSize, DocumentFieldFillType}}, accessors::v0::DataContractV0Getters}, prelude::DataContract};

#[derive(MockComponent)]
pub(crate) struct CreateStrategyScreen {
    component: Paragraph,
}

impl CreateStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let mut combined_spans = Vec::new();
        if let Some(strategy_key) = &app_state.current_strategy {
            // Append the current strategy name in bold to combined_spans
            combined_spans.push(TextSpan::new(&format!("--- {} ---", strategy_key)).bold());
            combined_spans.push(TextSpan::new(" "));
        
            if let Some(strategy) = app_state.available_strategies.get(strategy_key) {
                for (key, value) in &strategy.description {
                    // Append key in normal style
                    combined_spans.push(TextSpan::new(&format!("{}: ", key)).bold());
                    
                    // Append value in italic style
                    combined_spans.push(TextSpan::new(&format!("{}",value)));
                }
            } else {
                // Handle the case where the strategy_key doesn't exist in available_strategies
                combined_spans.push(TextSpan::new("Error: current strategy not found in available strategies."));
            }
        } else {
            // Handle the case where app_state.current_strategy is None
            combined_spans.push(TextSpan::new("No strategy loaded.").bold());
        }
        
        CreateStrategyScreen {
            component: Paragraph::default().text(combined_spans.as_ref()),
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
                    description: "Go Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'a',
                    description: "Add new empty strategy",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'l',
                    description: "Load an existing strategy",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "Rename current strategy",
                    key_type: KeyType::Command,
                },
                // to do: add "e" for edit and navigate to the edit options below
                CommandPalletKey {
                    key: 'c',
                    description: "Edit Contracts field",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'o',
                    description: "Edit Operations field",
                    key_type: KeyType::Command,
                },
                // CommandPalletKey {
                //     key: 's',
                //     description: "Edit Start Identities field",
                //     key_type: KeyType::Command,
                // },
                // CommandPalletKey {
                //     key: 'i',
                //     description: "Edit Identity Insertions field",
                //     key_type: KeyType::Command,
                // },
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
            Event::Keyboard(KeyEvent {
                code: Key::Char('o'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(SelectOperationType)),
            // Event::Keyboard(KeyEvent {
            //     code: Key::Char('s'),
            //     modifiers: KeyModifiers::NONE,
            // }) => Some(Message::ExpectingInput(EditStartIdentities)),
            // Event::Keyboard(KeyEvent {
            //     code: Key::Char('i'),
            //     modifiers: KeyModifiers::NONE,
            // }) => Some(Message::ExpectingInput(EditIdentityInserts)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(LoadStrategy)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(RenameStrategy)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::AddNewStrategy),
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
                    .title("Select a contract. Navigate with your arrow keys and press ENTER to select. Press 'q' to go back.", Alignment::Center)
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

impl Component<Message, NoUserEvent> for EditContractsStruct {
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
                Some(Message::AddStrategyContract(self.selected_index))
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

#[derive(MockComponent)]
pub(crate) struct RenameStrategyStruct {
    component: Input,
    old: String,
}

impl RenameStrategyStruct {
    pub(crate) fn new(app_state: &mut AppState) -> Self {
        if app_state.current_strategy.is_none() {
            app_state.current_strategy = Some("new_strategy".to_string());
            app_state.available_strategies.insert("new_strategy".to_string(), default_strategy_details());
        }
        let old = app_state.current_strategy.clone().unwrap();
        Self {
            component: Input::default()
                .title("Type the new name for the strategy and hit ENTER", Alignment::Center),
            old: old,
        }
    }
}

impl Component<Message, NoUserEvent> for RenameStrategyStruct {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                match self.component.perform(cmd) {
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        Some(Message::RenameStrategy(self.old.clone(), s))
                    }
                    CmdResult::Submit(State::None) => Some(Message::ReloadScreen),
                    _ => Some(Message::Redraw),
                }
            }
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct LoadStrategyStruct {
    component: List,
    selected_index: usize,
}

impl LoadStrategyStruct {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let strategies = &app_state.available_strategies;
                
        let mut rows = TableBuilder::default();
        for (name, _) in strategies.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select a Strategy. Navigate with your arrow keys and press ENTER to select. Press 'q' to go back.", Alignment::Center)
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

impl Component<Message, NoUserEvent> for LoadStrategyStruct {
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
                Some(Message::LoadStrategy(self.selected_index))
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
            "IdentityTopUp".to_string(), 
            "IdentityUpdate".to_string(), 
            "IdentityWithdrawal".to_string(),
            "ContractCreate".to_string(),
            "ContractUpdate".to_string(),
            "IdentityTransfer".to_string(),
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
                        Some(Message::DocumentOp(
                            contract.clone(), 
                            doc_type.clone(),
                            action 
                        ))
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
