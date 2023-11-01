//! Confirm selected strategy

use tui_realm_stdlib::Paragraph;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::TextSpan};

use crate::{app::{Message, state::AppState, strategies::Description}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct ConfirmStrategyScreen {
    component: Paragraph,
}

impl ConfirmStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let selected_strategy = &app_state.selected_strategy;
        let introduction = TextSpan::new("Confirm you would like to run the strategy:");
        let strategy_name = TextSpan::new(selected_strategy.clone().unwrap_or_default()).bold();

        let description_spans = match &app_state.selected_strategy {
            Some(strategy_key) => {
                if let Some(strategy) = app_state.available_strategies.get(strategy_key) {
                    strategy.strategy_description().iter()
                        .map(|(key, value)| TextSpan::new(&format!("{}: {}", key, value)))
                        .collect()
                } else {
                    // Handle the case where the strategy_key doesn't exist in available_strategies
                    vec![TextSpan::new("Error: strategy not found.")]
                }
            },
            None => {
                // Handle the case where app_state.selected_strategy is None
                vec![TextSpan::new("No selected strategy.")]
            }
        };

        let mut combined_spans = Vec::new();
        combined_spans.push(introduction);
        combined_spans.push(TextSpan::new(" "));
        combined_spans.push(strategy_name);
        combined_spans.extend(description_spans);

        ConfirmStrategyScreen {
            component: Paragraph::default().text(combined_spans.as_ref())
        }
    }
}

impl Component<Message, NoUserEvent> for ConfirmStrategyScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct ConfirmStrategyScreenCommands {
    component: CommandPallet,
}

impl ConfirmStrategyScreenCommands {
    pub(crate) fn new() -> Self {
        ConfirmStrategyScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Go Back",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'c',
                    description: "Confirm",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for ConfirmStrategyScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) => {
                // TO DO: Run the strategy now
                None
            },
            _ => None,
        }
    }
}