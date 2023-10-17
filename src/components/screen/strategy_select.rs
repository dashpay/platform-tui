//! Select strategy

use dpp::{version::PlatformVersion, tests::json_document::json_document_to_created_contract};
use tui_realm_stdlib::List;
use tuirealm::{MockComponent, Component, NoUserEvent, Event, event::{KeyEvent, Key, KeyModifiers}};
use strategy_tests::{Strategy, frequency::Frequency};
use crate::{app::Message, mock_components::{CommandPallet, CommandPalletKey, KeyType}};

fn default_strategy_1() -> Strategy {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_created_contract(
        "supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    Strategy {
        contracts_with_updates: vec![(contract, None)],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        signer: None,
    }
}

fn default_strategy_2() -> Strategy {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_created_contract(
        "supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    Strategy {
        contracts_with_updates: vec![(contract, None)],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        signer: None,
    }
}

fn default_strategy_3() -> Strategy {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_created_contract(
        "supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    Strategy {
        contracts_with_updates: vec![(contract, None)],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        signer: None,
    }
}

#[derive(MockComponent)]
pub(crate) struct SelectStrategyScreen {
    component: List,
}

impl SelectStrategyScreen {
    pub(crate) fn new() -> Self {
        SelectStrategyScreen {
            component: List::default(),
        }
    }
}

impl Component<Message, NoUserEvent> for SelectStrategyScreen {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
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
                    description: "Back to Main",
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