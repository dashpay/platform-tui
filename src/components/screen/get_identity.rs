//! Get identity screen

//! Identity screen module.

use tui_realm_stdlib::Textarea;
use tuirealm::{
    command::{Cmd, CmdResult, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::{
    app::{InputType::Base58IdentityId, Message},
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
            component: Textarea::default().highlighted_str(">"),
        }
    }
}

impl Component<Message, NoUserEvent> for GetIdentityScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(
                KeyEvent { code: Key::Up, .. }
                | KeyEvent {
                    code: Key::Char('p'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Scroll(Direction::Up));
                Some(Message::Redraw)
            }
            Event::Keyboard(
                KeyEvent {
                    code: Key::Down, ..
                }
                | KeyEvent {
                    code: Key::Char('n'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Scroll(Direction::Down));
                Some(Message::Redraw)
            }
            _ => None,
        }
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
            }) => Some(Message::ExpectingInput(Base58IdentityId)),
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
        // TODO: should be a history item not hardcoded but it's useful for development
        completions.add_history_item("5PhRFRrWZc5Mj8NqtpHNXCmmEQkcZE8akyDkKhsUVD4k".to_owned());
        completions.add_history_item("test1".to_owned());
        completions.add_history_item("test12".to_owned());
        completions.add_history_item("test13".to_owned());
        completions.add_history_item("test14".to_owned());
        completions.add_history_item("test15".to_owned());
        completions.add_history_item("test16".to_owned());
        completions.add_history_item("test17".to_owned());
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
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        Some(Message::FetchIdentityById(s))
                    }
                    CmdResult::Submit(State::None) => Some(Message::ReloadScreen),
                    _ => Some(Message::Redraw),
                }
            }
            _ => None,
        }
    }
}
