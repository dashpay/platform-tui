//! Select strategy

use tui_realm_stdlib::List;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, command::{Cmd, Direction, Position}, props::{TableBuilder, TextSpan, Color, Borders, BorderType, Alignment}};
use crate::{app::{Message, state::AppState}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct SelectStrategyScreen {
    component: List,
    selected_index: usize,
}

impl SelectStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let strategies = &app_state.available_strategies;
                
        let mut rows = TableBuilder::default();
        for (name, _) in strategies.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        SelectStrategyScreen {
            component: List::default()
                    .borders(
                        Borders::default()
                            .modifiers(BorderType::Rounded)
                            .color(Color::Yellow),
                    )
                    .title("Select a Strategy. Navigate with your arrow keys and press ENTER to select.", Alignment::Center)
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

impl Component<Message, NoUserEvent> for SelectStrategyScreen {
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
                code: Key::PageDown, ..
            }) => {
                self.perform(Cmd::Scroll(Direction::Down));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => {
                self.perform(Cmd::Scroll(Direction::Up));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => {
                self.perform(Cmd::GoTo(Position::Begin));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent { 
                code: Key::End, .. 
            }) => {
                self.perform(Cmd::GoTo(Position::End));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent { 
                code: Key::Enter, .. 
            }) => {
                Some(Message::SelectedStrategy(self.selected_index))
            },
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct SelectStrategyScreenCommands {
    component: CommandPallet,
}

impl SelectStrategyScreenCommands {
    pub(crate) fn new() -> Self {
        SelectStrategyScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Strategies",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for SelectStrategyScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            _ => None,
        }
    }
}