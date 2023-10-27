//! Application logic module, includes model and screen ids.

mod identity;
pub(crate) mod state;
mod wallet;
pub(crate) mod error;
mod contract;
pub(crate) mod strategies;

use std::collections::BTreeMap;
use std::time::Duration;
use dashcore::{Address, Network, PrivateKey};
use dashcore::secp256k1::Secp256k1;

use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::DataContract;
use rs_dapi_client::DapiClient;
use strategy_tests::frequency::Frequency;
use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    props::PropPayload,
    terminal::TerminalBridge,
    tui::prelude::{Constraint, Direction, Layout},
    Application, ApplicationError, AttrValue, Attribute, EventListenerCfg, NoUserEvent, Sub,
    SubClause, SubEventClause, Update,
};
use crate::app::state::AppState;
use crate::app::wallet::{SingleKeyWallet, Wallet};


use crate::components::*;

use self::strategies::default_strategy_details;

fn make_screen_subs() -> Vec<Sub<ComponentId, NoUserEvent>> {
    vec![
        Sub::new(
            SubEventClause::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }),
            SubClause::IsMounted(ComponentId::CommandPallet),
        ),
        Sub::new(
            SubEventClause::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }),
            SubClause::IsMounted(ComponentId::CommandPallet),
        ),
        Sub::new(
            SubEventClause::Keyboard(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            }),
            SubClause::IsMounted(ComponentId::CommandPallet),
        ),
        Sub::new(
            SubEventClause::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            }),
            SubClause::IsMounted(ComponentId::CommandPallet),
        ),
    ]
}

/// Screen identifiers
#[derive(Debug, Hash, Clone, Eq, PartialEq, Copy, strum::AsRefStr)]
pub(super) enum Screen {
    Main,
    Identity,
    GetIdentity,
    Contracts,
    GetContract,
    Wallet,
    AddWallet,
    Strategies,
    LoadStrategy,
    CreateStrategy,
    ConfirmStrategy,
    StrategyContracts,
    StrategyOperations,
}

/// Component identifiers, required to triggers screen switch which involves mounting and
/// unmounting.
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(super) enum ComponentId {
    CommandPallet,
    Screen,
    Status,
    Breadcrumbs,
    Input,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::Main
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum InputType {
    Base58IdentityId,
    Base58ContractId,
    SeedPhrase,
    WalletPrivateKey,
    SelectedStrategy,
    AddContract,
    SelectOperationType,
    // EditStartIdentities,
    // EditIdentityInserts,
    LoadStrategy,
    RenameStrategy,
    Document,
    // IdentityTopUp,
    // IdentityUpdate,
    // IdentityWithdrawal,
    // ContractCreate,
    // ContractUpdate,
    // IdentityTransfer,
    DeleteStrategy,
}

#[derive(Debug, PartialEq)]
pub(super) enum Message {
    AppClose,
    NextScreen(Screen),
    PrevScreen,
    ReloadScreen,
    ExpectingInput(InputType),
    Redraw,
    FetchIdentityById(String),
    FetchContractById(String),
    AddSingleKeyWallet(String),
    UpdateLoadedWalletUTXOsAndBalance,
    SelectedStrategy(usize),
    AddStrategyContract(Vec<String>),
    RemoveContract,
    RenameStrategy(String, String),
    LoadStrategy(usize),
    SelectOperationType(usize),
    DocumentOp(DataContract, DocumentType, DocumentAction, u16, f64),
    RemoveOperation,
    AddNewStrategy,
    DuplicateStrategy,
    DeleteStrategy(usize),
}

pub(super) struct Model<'a> {
    /// Application
    pub app: Application<ComponentId, Message, NoUserEvent>,
    /// State
    pub state: AppState,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Current screen
    pub current_screen: Screen,
    /// Breadcrumbs
    pub breadcrumbs: Vec<Screen>,
    /// Used to draw to terminal
    pub terminal: TerminalBridge,
    /// DAPI Client
    pub dapi_client: &'a mut DapiClient,
}

impl<'a> Model<'a> {
    pub(crate) fn new(dapi_client: &'a mut DapiClient) -> Self {
        Self {
            app: Self::init_app().expect("Unable to init the application"),
            state: AppState::load(),
            quit: false,
            redraw: true,
            current_screen: Screen::Main,
            breadcrumbs: Vec::new(),
            terminal: TerminalBridge::new().expect("Cannot initialize terminal"),
            dapi_client,
        }
    }

    fn init_app() -> Result<Application<ComponentId, Message, NoUserEvent>, ApplicationError> {
        let mut app = Application::init(
            EventListenerCfg::default()
                .default_input_listener(Duration::from_millis(20))
                .poll_timeout(Duration::from_millis(10))
                .tick_interval(Duration::from_secs(1)),
        );

        // Mount components
        app.mount(
            ComponentId::Screen,
            Box::new(MainScreen::new()),
            make_screen_subs(),
        )?;
        app.mount(
            ComponentId::CommandPallet,
            Box::new(MainScreenCommands::new()),
            Vec::new(),
        )?;
        app.mount(
            ComponentId::Breadcrumbs,
            Box::new(Breadcrumbs::new()),
            Vec::new(),
        )?;
        app.mount(ComponentId::Status, Box::new(Status::new()), Vec::new())?;

        // Setting focus on the screen so it will react to events
        app.active(&ComponentId::CommandPallet)?;

        Ok(app)
    }

    pub fn view(&mut self) {
        self.terminal
            .raw_mut()
            .draw(|f| {
                // App layout: screen window, screen keys and status bar
                let outer_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [Constraint::Min(10), Constraint::Max(10), Constraint::Max(2)].as_ref(),
                    )
                    .split(f.size());

                // Status line layout
                let status_bar_layout = Layout::default()
                    .horizontal_margin(1)
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(20), Constraint::Max(20)].as_ref())
                    .split(outer_layout[2]);

                self.app.view(&ComponentId::Screen, f, outer_layout[0]);

                if self.app.mounted(&ComponentId::CommandPallet) {
                    self.app
                        .view(&ComponentId::CommandPallet, f, outer_layout[1]);
                } else if self.app.mounted(&ComponentId::Input) {
                    self.app.view(&ComponentId::Input, f, outer_layout[1]);
                }

                self.app
                    .view(&ComponentId::Breadcrumbs, f, status_bar_layout[0]);
                self.app.view(&ComponentId::Status, f, status_bar_layout[1]);
            })
            .expect("unable to render the application");
    }

    pub fn set_screen(&mut self, screen: Screen) {
        match screen {
            Screen::Main => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(MainScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(MainScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::Identity => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(IdentityScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(IdentityScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::Contracts => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(ContractScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(ContractScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::GetIdentity => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(GetIdentityScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(GetIdentityScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::GetContract => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(GetContractScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(GetContractScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::Wallet => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(WalletScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(WalletScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
            Screen::AddWallet => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(AddWalletScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(AddWalletScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::Strategies => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategiesScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategiesScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::LoadStrategy => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(LoadStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(LoadStrategyScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::CreateStrategy => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(CreateStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(CreateStrategyScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::ConfirmStrategy => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(ConfirmStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(ConfirmStrategyScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::StrategyContracts => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyContractsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategyContractsScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::StrategyOperations => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyOperationsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategyOperationsScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
        }
        self.app
            .attr(
                &ComponentId::Breadcrumbs,
                Attribute::Text,
                AttrValue::String(
                    self.breadcrumbs
                        .iter()
                        .chain(std::iter::once(&self.current_screen))
                        .map(AsRef::as_ref)
                        .fold(String::new(), |mut acc, segment| {
                            acc.push_str(segment);
                            acc.push_str(" / ");
                            acc
                        }),
                ),
            )
            .expect("cannot set breadcrumbs");
        self.app
            .active(&ComponentId::CommandPallet)
            .expect("cannot set active");
    }
}

impl Update<Message> for Model<'_> {
    fn update(&mut self, message: Option<Message>) -> Option<Message> {
        if let Some(message) = message {
            // Set redraw
            self.redraw = true;
            // Match message
            match message {
                Message::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                Message::NextScreen(s) => {
                    self.breadcrumbs.push(self.current_screen);
                    self.current_screen = s;
                    self.set_screen(s);
                    None
                }
                Message::PrevScreen => {
                    let screen = self
                        .breadcrumbs
                        .pop()
                        .expect("must not be triggered on the main screen");
                    self.current_screen = screen;
                    self.set_screen(screen);
                    None
                }
                Message::ReloadScreen => {
                    self.set_screen(self.current_screen);
                    None
                }
                Message::ExpectingInput(input_type) => {
                    if self.app.mounted(&ComponentId::Input) {
                        self.app
                            .umount(&ComponentId::Input)
                            .expect("unable to umount Input");
                    }
                    
                    self.app
                        .umount(&ComponentId::CommandPallet)
                        .expect("unable to umount component");

                    match input_type {
                        InputType::Base58IdentityId => {
                            self.app
                                .mount(ComponentId::Input, Box::new(IdentityIdInput::new()), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::Base58ContractId => {
                            self.app
                                .mount(ComponentId::Input, Box::new(ContractIdInput::new()), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::SeedPhrase => {
                            self.app
                                .mount(ComponentId::Input, Box::new(PrivateKeyInput::new()), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::WalletPrivateKey => {
                            self.app
                                .mount(ComponentId::Input, Box::new(PrivateKeyInput::new()), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::SelectedStrategy => {
                            self.app
                                .mount(ComponentId::Input, Box::new(StrategySelect::new(&self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::AddContract => {
                            self.app
                                .mount(ComponentId::Input, Box::new(AddContractStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::SelectOperationType => {
                            self.app
                                .mount(ComponentId::Input, Box::new(SelectOperationTypeStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        // InputType::EditStartIdentities => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(EditStartIdentitiesStruct::new(&self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::EditIdentityInserts => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(EditIdentityInsertsStruct::new(&self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        InputType::LoadStrategy => {
                            self.app
                                .mount(ComponentId::Input, Box::new(LoadStrategyStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::DeleteStrategy => {
                            self.app
                                .mount(ComponentId::Input, Box::new(DeleteStrategyStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::RenameStrategy => {
                            if self.state.current_strategy.is_some() {
                                self.app
                                .mount(ComponentId::Input, Box::new(RenameStrategyStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                            } else {
                                self.app
                                    .mount(ComponentId::CommandPallet, Box::new(LoadStrategyScreenCommands::new()), vec![])
                                    .expect("unable to mount component");
                                self.app
                                    .active(&ComponentId::CommandPallet)
                                    .expect("cannot set active");
                                return None
                            }
                        }
                        InputType::Document => {
                            self.app
                                .mount(ComponentId::Input, Box::new(DocumentStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        // InputType::IdentityTopUp => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(IdentityTopUpStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::IdentityUpdate => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(IdentityUpdateStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::IdentityWithdrawal => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(IdentityWithdrawalStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::ContractCreate => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(ContractCreateStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::ContractUpdate => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(ContractUpdateStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                        // InputType::IdentityTransfer => {
                        //     self.app
                        //         .mount(ComponentId::Input, Box::new(IdentityTransferStruct::new(&mut self.state)), vec![])
                        //         .expect("unable to mount component");
                        // }
                    }


                    self.app
                        .active(&ComponentId::Input)
                        .expect("cannot set active");

                    None
                }
                Message::Redraw => None,
                Message::FetchIdentityById(s) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(GetIdentityScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");
                    let identity_spans =
                        identity::fetch_identity_bytes_by_b58_id(self.dapi_client, s)
                            .and_then(|bytes| identity::identity_bytes_to_spans(&bytes))
                            .expect("TODO error handling");

                    self.app
                        .attr(
                            &ComponentId::Screen,
                            Attribute::Text,
                            AttrValue::Payload(PropPayload::Vec(identity_spans)),
                        )
                        .unwrap();
                    None
                }
                Message::UpdateLoadedWalletUTXOsAndBalance => {
                    // self.app
                    //     .attr(
                    //         &ComponentId::Screen,
                    //         Attribute::Text,
                    //         AttrValue::Payload(PropPayload::Vec(identity_spans)),
                    //     )
                    //     .unwrap();
                    None
                }
                Message::FetchContractById(s) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(GetIdentityScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");
                    let identity_spans =
                        identity::fetch_identity_bytes_by_b58_id(self.dapi_client, s)
                            .and_then(|bytes| identity::identity_bytes_to_spans(&bytes))
                            .expect("TODO error handling");

                    self.app
                        .attr(
                            &ComponentId::Screen,
                            Attribute::Text,
                            AttrValue::Payload(PropPayload::Vec(identity_spans)),
                        )
                        .unwrap();
                    None
                }
                Message::AddSingleKeyWallet(private_key) => {
                    let private_key = if private_key.len() == 64 {
                        // hex
                        let bytes = hex::decode(private_key).expect("expected hex");
                        PrivateKey::from_slice(bytes.as_slice(), Network::Testnet).expect("expected private key")
                    } else {
                        PrivateKey::from_wif(private_key.as_str()).expect("expected WIF key")
                    };

                    let secp = Secp256k1::new();
                    let public_key = private_key.public_key(&secp);
                    //todo: make the network be part of state
                    let address = Address::p2pkh(&public_key, Network::Testnet);
                    let wallet = Wallet::SingleKeyWallet(SingleKeyWallet {
                        private_key: private_key.inner.secret_bytes(),
                        public_key: public_key.to_bytes(),
                        address: address.to_string(),
                        utxos: Default::default(),
                    });

                    self.state.loaded_wallet = Some(wallet.into());
                    self.state.save();
                    None
                },
                Message::SelectedStrategy(index) => {
                    let strategy = self.state.available_strategies.iter().nth(index).map(|(k, _)| k.clone()).unwrap_or_default();
                    self.state.selected_strategy = Some(strategy);
                    self.state.save();
                    Some(Message::NextScreen(Screen::ConfirmStrategy))
                },
                Message::AddStrategyContract(contracts) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(StrategyContractsScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");
                        
                    let current = self.state.current_strategy.clone().unwrap_or_default();
                    if let Some(strategy) = self.state.available_strategies.get_mut(&current) {
                        if let Some(first_contract_key) = contracts.get(0) {
                            if let Some(first_contract) = self.state.known_contracts.get(first_contract_key) {
                                if contracts.len() == 1 {
                                    strategy.strategy.contracts_with_updates.push((first_contract.clone(), None));
                                } else {
                                    let mut contract_updates = BTreeMap::new();
                        
                                    for (index, contract_key) in contracts.iter().enumerate().skip(1) {
                                        if let Some(contract) = self.state.known_contracts.get(contract_key) {
                                            contract_updates.insert(index as u64, contract.clone());
                                        }
                                    }
                        
                                    strategy.strategy.contracts_with_updates.push((first_contract.clone(), Some(contract_updates)));
                                }
                            }
                        }
                            
                        let description_entry = strategy.description.entry("contracts_with_updates".to_string()).or_insert("".to_string());
                        
                        let new_contracts_formatted = contracts.join("::");
                        
                        if !description_entry.is_empty() {
                            description_entry.push_str(", ");
                        }
                        description_entry.push_str(&new_contracts_formatted);
                    }
                    self.state.save();

                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyContractsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
                Message::RemoveContract => {
                    let current_name = self.state.current_strategy.clone().unwrap();
                    let current_strategy_details = self.state.available_strategies.get_mut(&current_name).unwrap();
                    current_strategy_details.strategy.contracts_with_updates.pop();
                    let description_contracts = current_strategy_details.description.get_mut("contracts_with_updates").unwrap();
                    let mut values: Vec<&str> = description_contracts.split(',').collect();
                    values.pop();
                    *description_contracts = values.join(",");
                
                    self.state.save();
                
                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(CreateStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                
                    Some(Message::Redraw)
                },
                Message::RenameStrategy(old, new) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(LoadStrategyScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    let strategy = self.state.available_strategies.get(&old);
                    self.state.current_strategy = Some(new.clone());
                    self.state.available_strategies.insert(new, strategy.unwrap().clone());
                    self.state.available_strategies.remove(&old);

                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(LoadStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
                Message::LoadStrategy(index) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(LoadStrategyScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    let strategy = self.state.available_strategies.iter().nth(index).map(|(k, _)| k.clone()).unwrap_or_default();
                    self.state.current_strategy = Some(strategy);
                    self.state.save();
    
                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(LoadStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
                Message::SelectOperationType(index) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(CreateStrategyScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    let op_types = vec![
                        "Document", 
                        "IdentityTopUp", 
                        "IdentityUpdate", 
                        "IdentityWithdrawal",
                        "ContractCreate",
                        "ContractUpdate",
                        "IdentityTransfer",
                    ];
                                
                    match op_types.get(index) {
                        Some(&"Document") => Some(Message::ExpectingInput(InputType::Document)),
                        // Some(&"IdentityTopUp") => Some(Message::ExpectingInput(InputType::IdentityTopUp)),
                        // Some(&"IdentityUpdate") => Some(Message::ExpectingInput(InputType::IdentityUpdate)),
                        // Some(&"IdentityWithdrawal") => Some(Message::ExpectingInput(InputType::IdentityWithdrawal)),
                        // Some(&"ContractCreate") => Some(Message::ExpectingInput(InputType::ContractCreate)),
                        // Some(&"ContractUpdate") => Some(Message::ExpectingInput(InputType::ContractUpdate)),
                        // Some(&"IdentityTransfer") => Some(Message::ExpectingInput(InputType::IdentityTransfer)),
                        _ => None,
                    }
                },
                Message::DocumentOp(contract, doc_type, action, tpbr, cpb) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(StrategyOperationsScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    let doc_op = DocumentOp {
                        contract: contract.clone(),
                        document_type: doc_type.clone(),
                        action: action.clone()
                    };
                    let mut op_vec = Vec::new();
                    op_vec.push(doc_op.clone());
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy_details = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    let mut current_strategy = current_strategy_details.strategy.clone();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::Document(doc_op),
                        frequency: Frequency {
                            times_per_block_range: 1..tpbr,
                            chance_per_block: Some(cpb),
                        },
                    });

                    let action_name = match action {
                        DocumentAction::DocumentActionInsertRandom(_, _) => "InsertRandom",
                        DocumentAction::DocumentActionDelete => "Delete",
                        DocumentAction::DocumentActionReplace => "Replace",
                        DocumentAction::DocumentActionInsertSpecific(_, _, _, _) => "InsertSpecific",
                    };

                    let op_description = format!("DocumentOp::{}::{}::{}::{}", 
                        doc_type.name(), 
                        action_name,
                        format!("MTPB={}",tpbr),
                        format!("CPB={}",cpb),
                    );
                    
                    let description_entry = current_strategy_details.description.entry("operations".to_string()).or_insert("".to_string());
                    if description_entry.is_empty() || description_entry == "-" {
                        *description_entry = op_description;
                    } else {
                        description_entry.push_str(&format!(", {}", op_description));
                    }

                    self.state.save();

                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyOperationsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
                Message::RemoveOperation => {
                    let current_name = self.state.current_strategy.clone().unwrap();
                    let current_strategy_details = self.state.available_strategies.get_mut(&current_name).unwrap();
                    current_strategy_details.strategy.operations.pop();
                    let description_contracts = current_strategy_details.description.get_mut("operations").unwrap();
                    let mut values: Vec<&str> = description_contracts.split(',').collect();
                    values.pop();
                    *description_contracts = values.join(",");
                
                    self.state.save();
                
                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyOperationsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                
                    Some(Message::Redraw)
                },
                Message::AddNewStrategy => {
                    self.state.available_strategies.insert("new_strategy".to_string(), default_strategy_details());
                    self.state.current_strategy = Some("new_strategy".to_string());
                    self.state.save();

                    self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(LoadStrategyScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::ExpectingInput(InputType::RenameStrategy))
                },
                Message::DuplicateStrategy => {
                    if self.state.current_strategy.is_some() {
                        let current = self.state.available_strategies.get(&self.state.current_strategy.clone().unwrap_or_default()).unwrap();
                        self.state.available_strategies.insert("new_clone".to_string(), current.clone());
                        self.state.current_strategy = Some("new_clone".to_string());
                        self.state.save();

                        self.app
                        .remount(
                            ComponentId::Screen,
                            Box::new(LoadStrategyScreen::new(&self.state)),
                            make_screen_subs(),
                        )
                        .expect("unable to remount screen");

                        Some(Message::Redraw)
                    } else { None }
                },
                Message::DeleteStrategy(index) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(StrategiesScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    self.state.current_strategy = None;
                    if let Some(key) = self.state.available_strategies.keys().nth(index).cloned() {
                        self.state.available_strategies.remove(&key);
                    }
                    self.state.save();

                    self.app

                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategiesScreen::new()),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
            }
        } else {
            None
        }
    }
}
