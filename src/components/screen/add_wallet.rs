//! Add wallet screen

use tui_realm_stdlib::Textarea;
use tuirealm::{event::{Key, KeyEvent, KeyModifiers}, Component, Event, MockComponent, NoUserEvent, State, StateValue};
use tuirealm::command::CmdResult;

use crate::{
    app::Message,
    mock_components::{CommandPallet, CommandPalletKey, KeyType},
};
use crate::app::InputType::{SeedPhrase, WalletPrivateKey};
use crate::mock_components::{CompletingInput, HistoryCompletionEngine, key_event_to_cmd};

#[derive(MockComponent)]
pub(crate) struct AddWalletScreen {
    component: Textarea,
}

impl AddWalletScreen {
    pub(crate) fn new() -> Self {
        AddWalletScreen {
            component: Textarea::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for AddWalletScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct AddWalletScreenCommands {
    component: CommandPallet,
}

impl AddWalletScreenCommands {
    pub(crate) fn new() -> Self {
        AddWalletScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Wallet screen",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'p',
                    description: "Add by private key",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 's',
                    description: "Add by seed",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for AddWalletScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                                code: Key::Char('q'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                                code: Key::Char('p'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::ExpectingInput(WalletPrivateKey)),
            Event::Keyboard(KeyEvent {
                                code: Key::Char('s'),
                                modifiers: KeyModifiers::NONE,
                            }) => Some(Message::ExpectingInput(SeedPhrase)),
            _ => None,
        }
    }
}


#[derive(MockComponent)]
pub(crate) struct PrivateKeyInput {
    component: CompletingInput<HistoryCompletionEngine>,
}

impl PrivateKeyInput {
    pub(crate) fn new() -> Self {
        let mut completions = HistoryCompletionEngine::default();
        // TODO: should be a history item not hardcoded but it's useful for development
        completions.add_history_item("5PhRFRrWZc5Mj8NqtpHNXCmmEQkcZE8akyDkKhsUVD4k".to_owned());
        Self {
            component: CompletingInput::new(completions, "hex private key"),
        }
    }
}

impl Component<Message, NoUserEvent> for PrivateKeyInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                match self.component.perform(cmd) {
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        Some(Message::AddSingleKeyWallet(s))
                    }
                    CmdResult::Submit(State::None) => Some(Message::ReloadScreen),
                    _ => Some(Message::Redraw),
                }
            }
            _ => None,
        }
    }
}
