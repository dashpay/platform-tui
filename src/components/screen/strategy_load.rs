//! Create strategy

use tui_realm_stdlib::{Input, List, Paragraph};
use tuirealm::{
    command::{Cmd, CmdResult, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Alignment, TableBuilder, TextSpan},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::app::InputType::{DeleteStrategy, LoadStrategy, RenameStrategy};
use crate::{
    app::{
        state::AppState,
        strategies::{default_strategy, Description},
        Message, Screen,
    },
    mock_components::{key_event_to_cmd, CommandPallet, CommandPalletKey, KeyType},
};

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
                for (key, value) in &strategy.strategy_description() {
                    combined_spans.push(TextSpan::new(&format!("  {}:", key)).bold());
                    combined_spans.push(TextSpan::new(&format!("    {}", value)));
                }
            } else {
                // Handle the case where the strategy_key doesn't exist in available_strategies
                combined_spans.push(TextSpan::new(
                    "Error: current strategy not found in available strategies.",
                ));
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
    state: AppState,
}

impl LoadStrategyScreenCommands {
    pub(crate) fn new(app_state: &AppState) -> Self {
        if app_state.current_strategy.is_some() {
            Self {
                component: CommandPallet::new(vec![
                    CommandPalletKey {
                        key: 'q',
                        description: "Back",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'n',
                        description: "New",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'l',
                        description: "Load",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'r',
                        description: "Rename",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'e',
                        description: "Edit",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'c',
                        description: "Clone",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'd',
                        description: "Delete",
                        key_type: KeyType::Command,
                    },
                ]),
                state: app_state.clone(),
            }
        } else {
            Self {
                component: CommandPallet::new(vec![
                    CommandPalletKey {
                        key: 'q',
                        description: "Back",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'n',
                        description: "New",
                        key_type: KeyType::Command,
                    },
                    CommandPalletKey {
                        key: 'l',
                        description: "Load",
                        key_type: KeyType::Command,
                    },
                ]),
                state: app_state.clone(),
            }
        }
    }
}

impl Component<Message, NoUserEvent> for LoadStrategyScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        if self.state.current_strategy.is_some() {
            match ev {
                Event::Keyboard(KeyEvent {
                    code: Key::Char('q'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::PrevScreen),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('e'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::NextScreen(Screen::CreateStrategy)),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('r'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::ExpectingInput(RenameStrategy)),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('c'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::DuplicateStrategy),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('d'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::ExpectingInput(DeleteStrategy)),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('l'),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if self.state.available_strategies.is_empty() {
                        None
                    } else {
                        Some(Message::ExpectingInput(LoadStrategy))
                    }
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Char('n'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::AddNewStrategy),
                _ => None,
            }
        } else {
            match ev {
                Event::Keyboard(KeyEvent {
                    code: Key::Char('q'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::PrevScreen),
                Event::Keyboard(KeyEvent {
                    code: Key::Char('l'),
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if self.state.available_strategies.is_empty() {
                        None
                    } else {
                        Some(Message::ExpectingInput(LoadStrategy))
                    }
                }
                Event::Keyboard(KeyEvent {
                    code: Key::Char('n'),
                    modifiers: KeyModifiers::NONE,
                }) => Some(Message::AddNewStrategy),
                _ => None,
            }
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
                .title(
                    "Select a Strategy. Press 'q' to go back.",
                    Alignment::Center,
                )
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
                let max_index = self.component.states.list_len - 2;
                if self.selected_index < max_index {
                    self.selected_index = self.selected_index + 1;
                    self.perform(Cmd::Move(Direction::Down));
                }
                Some(Message::Redraw)
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.perform(Cmd::Move(Direction::Up));
                }
                Some(Message::Redraw)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => Some(Message::LoadStrategy(self.selected_index)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(Message::ReloadScreen),
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
            app_state
                .available_strategies
                .insert("new_strategy".to_string(), default_strategy());
        }
        let old = app_state.current_strategy.clone().unwrap();
        Self {
            component: Input::default().title(
                "Type the new name for the strategy and hit ENTER",
                Alignment::Center,
            ),
            old,
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
pub(crate) struct DeleteStrategyStruct {
    component: List,
    selected_index: usize,
}

impl DeleteStrategyStruct {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let strategies = &app_state.available_strategies;

        let mut rows = TableBuilder::default();
        for (name, _) in strategies.iter() {
            rows.add_col(TextSpan::from(name));
            rows.add_row();
        }

        Self {
            component: List::default()
                .title(
                    "Select a Strategy. Press 'q' to go back.",
                    Alignment::Center,
                )
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

impl Component<Message, NoUserEvent> for DeleteStrategyStruct {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                let max_index = self.component.states.list_len - 2;
                if self.selected_index < max_index {
                    self.selected_index = self.selected_index + 1;
                    self.perform(Cmd::Move(Direction::Down));
                }
                Some(Message::Redraw)
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.perform(Cmd::Move(Direction::Up));
                }
                Some(Message::Redraw)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => Some(Message::DeleteStrategy(self.selected_index)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(Message::ReloadScreen),
            _ => None,
        }
    }
}
