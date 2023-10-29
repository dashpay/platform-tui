//! Strategy Start Identities screen


use tui_realm_stdlib::{Paragraph, List};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Alignment}, command::{Cmd, Direction}};

use crate::{app::{Message, state::AppState, strategies::default_strategy_details}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};
use crate::app::InputType::StartIdentities;

#[derive(MockComponent)]
pub(crate) struct StrategyStartIdentitiesScreen {
    component: Paragraph,
}

impl StrategyStartIdentitiesScreen {
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

        Self {
            component: Paragraph::default().text(combined_spans.as_ref())
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyStartIdentitiesScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategyStartIdentitiesScreenCommands {
    component: CommandPallet,
}

impl StrategyStartIdentitiesScreenCommands {
    pub(crate) fn new() -> Self {
        Self {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'a',
                    description: "Add/Edit",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "Remove",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategyStartIdentitiesScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(StartIdentities)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::RemoveStartIdentities),
            _ => None,
        }
    }
}

pub enum StartIdentitiesSelectionState {
    SelectCount,
    SelectKeyCount { count: u16 },
}

#[derive(MockComponent)]
pub(crate) struct StartIdentitiesStruct {
    component: List,
    selected_index: usize,
    selection_state: StartIdentitiesSelectionState,
}

impl StartIdentitiesStruct {
    fn update_component_for_key_count(&mut self) {
        self.selected_index = 0;
        let options = vec!["2","3","4","5","10","20","32"];
        let mut rows = TableBuilder::default();
        for option in options {
            rows.add_col(TextSpan::from(option));
            rows.add_row();
        }
        self.component = List::default()
            .title("Select number of keys per identity.", Alignment::Center)
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
                
        let options = vec!["1","10","100","1000","10000","65535"];
        let mut rows = TableBuilder::default();
        for option in options {
            rows.add_col(TextSpan::from(option));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select the number of Identities.", Alignment::Center)
                    .scroll(true)
                    .highlighted_str("> ")
                    .rewind(true)
                    .step(1)
                    .rows(rows.build())
                    .selected_line(0),
            selected_index: 0,
            selection_state: StartIdentitiesSelectionState::SelectCount,
        }
    }
}

impl Component<Message, NoUserEvent> for StartIdentitiesStruct {
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
                    StartIdentitiesSelectionState::SelectCount => {

                        let count: u16 = match self.selected_index {
                            0 => 1,
                            1 => 10,
                            2 => 100,
                            3 => 1000,
                            4 => 10000,
                            5 => 65535,
                            _ => panic!("index out of bounds for StartIdentitiesSelectionState::SelectCount")
                        };

                        self.selection_state = StartIdentitiesSelectionState::SelectKeyCount { count };
                        self.update_component_for_key_count();
                        
                        Some(Message::Redraw)                        
                    },
                    StartIdentitiesSelectionState::SelectKeyCount { count } => {
                        let key_count: u32 = match self.selected_index {
                            0 => 2,
                            1 => 3,
                            2 => 4,
                            3 => 5,
                            4 => 10,
                            5 => 20,
                            6 => 32,
                            _ => panic!("index out of bounds for StartIdentitiesSelectionState::SelectKeyCount")
                        };

                        Some(Message::StartIdentities(count.clone(), key_count))
                    },
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