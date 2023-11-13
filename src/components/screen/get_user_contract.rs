//! Get contract screen

use dpp::identifier::Identifier;
use dpp::platform_value::string_encoding::Encoding;
use tui_realm_stdlib::Textarea;
use tuirealm::{
    command::CmdResult,
    event::{Key, KeyEvent, KeyModifiers},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::{
    app::{InputType::Base58ContractId, Message},
    mock_components::{
        key_event_to_cmd, CommandPallet, CommandPalletKey, CompletingInput,
        HistoryCompletionEngine, KeyType,
    },
};

#[derive(MockComponent)]
pub(crate) struct GetUserContractScreen {
    component: Textarea,
}

impl GetUserContractScreen {
    pub(crate) fn new() -> Self {
        GetUserContractScreen {
            component: Textarea::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for GetUserContractScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct GetUserContractScreenCommands {
    component: CommandPallet,
}

impl GetUserContractScreenCommands {
    pub(crate) fn new() -> Self {
        GetUserContractScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Contract screen",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'i',
                    description: "Get by ID",
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

impl Component<Message, NoUserEvent> for GetUserContractScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::ExpectingInput(Base58ContractId)),
            _ => None,
        }
    }
}

#[derive(MockComponent)]
pub(crate) struct UserContractIdInput {
    component: CompletingInput<HistoryCompletionEngine>,
}

impl UserContractIdInput {
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

impl Component<Message, NoUserEvent> for UserContractIdInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                match self.component.perform(cmd) {
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        match Identifier::from_string(s.as_str(), Encoding::Base58) {
                            Ok(identifier) => {
                                Some(Message::FetchContractById("name".to_string(), identifier))
                            }
                            Err(e) => Some(Message::DisplayError(e.to_string())),
                        }
                    }
                    CmdResult::Submit(State::None) => Some(Message::ReloadScreen),
                    _ => Some(Message::Redraw),
                }
            }
            _ => None,
        }
    }
}
