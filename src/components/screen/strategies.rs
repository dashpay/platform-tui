//! Strategies screen

use tuirealm::{MockComponent, Component, props::{TextSpan, TableBuilder, Alignment}, Event, NoUserEvent, event::{KeyEvent, KeyModifiers, Key}, command::{Cmd, Direction}};
use tui_realm_stdlib::{Paragraph, List};
use crate::app::{Message, Screen, state::AppState};
use crate::mock_components::{CommandPallet, CommandPalletKey, KeyType};
use crate::app::InputType::{SelectedStrategy, DeleteStrategy};

#[derive(MockComponent)]
pub(crate) struct StrategiesScreen {
    component: Paragraph,
}

impl StrategiesScreen {
    pub(crate) fn new() -> Self {
        StrategiesScreen {
            component: Paragraph::default()
                .text([TextSpan::new("Strategies management commands")].as_ref()),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategiesScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategiesScreenCommands {
    component: CommandPallet,
}

impl StrategiesScreenCommands {
    pub(crate) fn new() -> Self {
        StrategiesScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'r',
                    description: "Run",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Create and edit",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for StrategiesScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(SelectedStrategy)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::LoadStrategy)),
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct StrategySelect {
    component: List,
    selected_index: usize,
}

impl StrategySelect {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let strategies = &app_state.available_strategies;
                
        let mut rows = TableBuilder::default();
        for (name, _) in strategies.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                    .title("Select a Strategy. Press 'q' to go back.", Alignment::Center)
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

impl Component<Message, NoUserEvent> for StrategySelect {
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
                Some(Message::SelectedStrategy(self.selected_index))
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