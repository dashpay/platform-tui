//! Select strategy

use dpp::{version::PlatformVersion, tests::json_document::json_document_to_created_contract};
use tui_realm_stdlib::List;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, command::{Cmd, Direction, CmdResult, Position}, props::{TableBuilder, TextSpan, Color, Borders, BorderType, Alignment}};
use strategy_tests::{Strategy, frequency::Frequency};
use crate::{app::{Message, state::AppState}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct SelectStrategyScreen {
    component: List,
}

impl SelectStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let strategies = &app_state.available_strategies;
                
        let mut rows = TableBuilder::default();
        for (index, (name, _)) in strategies.iter().enumerate() {
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
                .title("Select a Strategy", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::LightYellow)
                .highlighted_str("> ")
                .rewind(true)
                .step(1)
                .rows(rows.build())
                .selected_line(0),
        }
    }
}

impl Component<Message, NoUserEvent> for SelectStrategyScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                self.perform(Cmd::Move(Direction::Down));
                Some(Message::Redraw)
            },
            Event::Keyboard(KeyEvent { 
                code: Key::Up, .. 
            }) => {
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
                self.perform(Cmd::Submit);
                Some(Message::Redraw)
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