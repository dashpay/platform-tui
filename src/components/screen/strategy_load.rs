//! Create strategy

use tui_realm_stdlib::{Paragraph, List};
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::{TextSpan, TableBuilder, Alignment}, command::{Cmd, Direction}};

use crate::{app::{Message, state::AppState, Screen}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};
use crate::app::InputType::LoadStrategy;

#[derive(MockComponent)]
pub(crate) struct LoadStrategyScreen {
    component: Paragraph,
}

impl LoadStrategyScreen {
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
            component: Paragraph::default().text(combined_spans.as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for LoadStrategyScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct LoadStrategyScreenCommands {
    component: CommandPallet,
}

impl LoadStrategyScreenCommands {
    pub(crate) fn new() -> Self {
        Self {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Go Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Create new",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'l',
                    description: "Load existing",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'e',
                    description: "Edit",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'd',
                    description: "Duplicate",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for LoadStrategyScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(LoadStrategy)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::AddNewStrategy),
            Event::Keyboard(KeyEvent {
                code: Key::Char('e'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::CreateStrategy)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::DuplicateStrategy),
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

