//! Get identity screen

//! Identity screen module.

use tui_realm_stdlib::{Paragraph, Table, Textarea};
use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::TextSpan,
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::{
    app::{Message, Screen},
    mock_components::{
        key_event_to_cmd, CommandPallet, CommandPalletKey, CompletingInput,
        HistoryCompletionEngine, KeyType,
    },
};

#[derive(MockComponent)]
pub(crate) struct GetIdentityScreen {
    component: Textarea,
}

impl GetIdentityScreen {
    pub(crate) fn new() -> Self {
        GetIdentityScreen {
            component: Textarea::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for GetIdentityScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct GetIdentityScreenCommands {
    component: CommandPallet,
}

impl GetIdentityScreenCommands {
    pub(crate) fn new() -> Self {
        GetIdentityScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Identity screen",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'i',
                    description: "Get by ID",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'h',
                    description: "Get by public key hashes",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'p',
                    description: "with proof",
                    key_type: KeyType::Toggle,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for GetIdentityScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput),
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.perform(Cmd::Type(c));
                Some(Message::Redraw)
            }
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct IdentityIdInput {
    component: CompletingInput<HistoryCompletionEngine>,
}

impl IdentityIdInput {
    pub(crate) fn new() -> Self {
        let mut completions = HistoryCompletionEngine::default();
        completions.add_history_item("5PhRFRrWZc5Mj8NqtpHNXCmmEQkcZE8akyDkKhsUVD4k".to_owned());
        Self {
            component: CompletingInput::new(completions, "base58 Identity ID"),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityIdInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                match self.component.perform(cmd) {
                    CmdResult::Changed(_) => Some(Message::Redraw),
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        Some(Message::FetchIdentityById(s))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
