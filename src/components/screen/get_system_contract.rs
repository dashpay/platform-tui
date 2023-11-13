//! Get contract screen

use dpp::data_contracts::{dpns_contract, SystemDataContract};
use dpp::system_data_contracts::dashpay_contract;
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
pub(crate) struct GetSystemContractScreen {
    component: Textarea,
}

impl GetSystemContractScreen {
    pub(crate) fn new() -> Self {
        GetSystemContractScreen {
            component: Textarea::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for GetSystemContractScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}

#[derive(MockComponent)]
pub(crate) struct GetSystemContractScreenCommands {
    component: CommandPallet,
}

impl GetSystemContractScreenCommands {
    pub(crate) fn new() -> Self {
        GetSystemContractScreenCommands {
            component: CommandPallet::new(vec![
                CommandPalletKey {
                    key: 'q',
                    description: "Back to Contract screen",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'p',
                    description: "Dashpay",
                    key_type: KeyType::Command,
                },
                CommandPalletKey {
                    key: 'n',
                    description: "DPNS",
                    key_type: KeyType::Command,
                },
            ]),
        }
    }
}

impl Component<Message, NoUserEvent> for GetSystemContractScreenCommands {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::PrevScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::FetchContractById(
                "dashpay".to_string(),
                dashpay_contract::ID_BYTES.into(),
            )),
            Event::Keyboard(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::FetchContractById(
                "dpns".to_string(),
                dpns_contract::ID_BYTES.into(),
            )),
            _ => None,
        }
    }
}
