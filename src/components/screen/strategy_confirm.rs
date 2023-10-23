//! Confirm selected strategy

use tui_realm_stdlib::Paragraph;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}, props::TextSpan};

use crate::{app::{Message, state::AppState}, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

#[derive(MockComponent)]
pub(crate) struct ConfirmStrategyScreen {
    component: Paragraph,
}

impl ConfirmStrategyScreen {
    pub(crate) fn new(app_state: &AppState) -> Self {
        // Fetch the strategy name based on the selected index from the app_state.
        // This assumes that app_state has a field like selected_strategy_index and 
        // available_strategies is a Vec or similar indexed collection.
        let selected_strategy = &app_state.selected_strategy;

        // Create the full message
        let message = format!("Confirm you would like to run the strategy: {}", selected_strategy.clone().unwrap_or_default());

        ConfirmStrategyScreen {
            component: Paragraph::default()
                .text([TextSpan::new(&message)].as_ref())
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
                    description: "Back to Strategies List",
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