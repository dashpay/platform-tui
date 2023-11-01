//! Create strategy

use tui_realm_stdlib::Paragraph;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::TextSpan};

use crate::{app::{Message, state::AppState, Screen, strategies::Description}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct CreateStrategyScreen {
    component: Paragraph,
}

impl CreateStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let mut combined_spans = Vec::new();
        if let Some(strategy_key) = &app_state.current_strategy {
            // Append the current strategy name in bold to combined_spans
            combined_spans.push(TextSpan::new(&format!("{}:", strategy_key)).bold());
        
            if let Some(strategy) = app_state.available_strategies.get(strategy_key) {
                for (key, value) in &strategy.strategy_description() {
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
                    description: "Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Contracts",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'o',
                    description: "Operations",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 's',
                    description: "Start Identities",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'i',
                    description: "Identities Inserts",
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
            }) => Some(Message::NextScreen(Screen::StrategyContracts)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('o'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::StrategyOperations)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::StartIdentities)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::NextScreen(Screen::IdentityInserts)),
            _ => None,
        }
    }
}



