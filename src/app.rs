//! Application logic module, includes model and screen ids.

mod contract;
pub(crate) mod error;
mod identity;
pub(crate) mod state;
mod wallet;
pub(crate) mod error;
mod contract;
pub(crate) mod strategies;

use std::cmp::min;
use std::collections::BTreeMap;
use std::time::Duration;
use dashcore::secp256k1::rand::SeedableRng;
use dashcore::secp256k1::rand::rngs::StdRng;
use dashcore::{Address, Network, PrivateKey};
use dashcore::secp256k1::Secp256k1;

use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::v0::random_document_type::{RandomDocumentTypeParameters, FieldTypeWeights, FieldMinMaxBounds};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use rand::Rng;
use rs_dapi_client::DapiClient;
use simple_signer::signer::SimpleSigner;
use strategy_tests::frequency::Frequency;
use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType, IdentityUpdateOp, DataContractUpdateOp};
use strategy_tests::transitions::create_identities_state_transitions;

use dashcore::secp256k1::Secp256k1;
use dashcore::{Address, Network, PrivateKey};

use std::time::Duration;

use crate::app::state::AppState;
use crate::app::wallet::{SingleKeyWallet, Wallet};
use rs_dapi_client::DapiClient;
use tokio::runtime::Runtime;
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

use self::strategies::{Description, default_strategy};

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
    IdentityInserts,
    StartIdentities,
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
    StartIdentities,
    LoadStrategy,
    RenameStrategy,
    Frequency(String),
    Document,
    IdentityUpdate,
    DeleteStrategy,
    ContractUpdate,
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
    Frequency(String, u16, f64),
    DocumentOp(DataContract, DocumentType, DocumentAction),
    IdentityTopUp,
    IdentityWithdrawal,
    IdentityTransfer,
    IdentityUpdate(String, u16),
    RemoveOperation,
    AddNewStrategy,
    DuplicateStrategy,
    DeleteStrategy(usize),
    RemoveIdentityInserts,
    StartIdentities(u16, u32),
    RemoveStartIdentities,
    ContractCreate,
    ContractUpdate(usize),
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
    /// Tokio runtime
    pub runtime: Runtime,
}

impl<'a> Model<'a> {
    pub(crate) fn new(dapi_client: &'a mut DapiClient) -> Self {
        let runtime = Runtime::new().expect("cannot start Tokio runtime");
        Self {
            app: Self::init_app().expect("Unable to init the application"),
            state: runtime.block_on(AppState::load()),
            quit: false,
            redraw: true,
            current_screen: Screen::Main,
            breadcrumbs: Vec::new(),
            terminal: TerminalBridge::new().expect("Cannot initialize terminal"),
            dapi_client,
            runtime,
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
                        Box::new(StrategiesScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategiesScreenCommands::new(&self.state)),
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
                        Box::new(LoadStrategyScreenCommands::new(&self.state)),
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
            Screen::IdentityInserts => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyIdentityInsertsScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategyIdentityInsertsScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            },
            Screen::StartIdentities => {
                self.app
                    .remount(
                        ComponentId::Screen,
                        Box::new(StrategyStartIdentitiesScreen::new(&self.state)),
                        make_screen_subs(),
                    )
                    .expect("unable to remount screen");
                self.app
                    .remount(
                        ComponentId::CommandPallet,
                        Box::new(StrategyStartIdentitiesScreenCommands::new()),
                        Vec::new(),
                    )
                    .expect("unable to remount screen");
            }
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
                        InputType::StartIdentities => {
                            self.app
                                .mount(ComponentId::Input, Box::new(StartIdentitiesStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
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
                                    .mount(ComponentId::CommandPallet, Box::new(LoadStrategyScreenCommands::new(&self.state)), vec![])
                                    .expect("unable to mount component");
                                self.app
                                    .active(&ComponentId::CommandPallet)
                                    .expect("cannot set active");
                                return None
                            }
                        }
                        InputType::Frequency(field) => {
                            self.app
                                .mount(ComponentId::Input, Box::new(FrequencyStruct::new(field)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::Document => {
                            self.app
                                .mount(ComponentId::Input, Box::new(DocumentStruct::new(&mut self.state)), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::IdentityUpdate => {
                            self.app
                                .mount(ComponentId::Input, Box::new(IdentityUpdateStruct::new()), vec![])
                                .expect("unable to mount component");
                        }
                        InputType::ContractUpdate => {
                            self.app
                                .mount(ComponentId::Input, Box::new(ContractUpdateStruct::new()), vec![])
                                .expect("unable to mount component");
                        }
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
                    let identity_spans = self
                        .runtime
                        .block_on(identity::fetch_identity_bytes_by_b58_id(
                            self.dapi_client,
                            s,
                        ))
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
                    let identity_spans = self
                        .runtime
                        .block_on(identity::fetch_identity_bytes_by_b58_id(
                            self.dapi_client,
                            s,
                        ))
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
                        PrivateKey::from_slice(bytes.as_slice(), Network::Testnet)
                            .expect("expected private key")
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
                                    strategy.contracts_with_updates.push((first_contract.clone(), None));
                                } else {
                                    let mut contract_updates = BTreeMap::new();
                        
                                    for (index, contract_key) in contracts.iter().enumerate().skip(1) {
                                        if let Some(contract) = self.state.known_contracts.get(contract_key) {
                                            contract_updates.insert(index as u64, contract.clone());
                                        }
                                    }
                        
                                    strategy.contracts_with_updates.push((first_contract.clone(), Some(contract_updates)));
                                }
                            }
                        }
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
                    current_strategy_details.contracts_with_updates.pop();
                
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
                            Box::new(LoadStrategyScreenCommands::new(&self.state)),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    let strategy = self.state.available_strategies.get(&old);
                    self.state.current_strategy = Some(new.clone());
                    self.state.available_strategies.insert(new.clone(), strategy.unwrap().clone());
                    if new != old {
                        self.state.available_strategies.remove(&old);
                    }

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
                Message::LoadStrategy(index) => {

                    let strategy = self.state.available_strategies.iter().nth(index).map(|(k, _)| k.clone()).unwrap_or_default();
                    self.state.current_strategy = Some(strategy);
                    self.state.save();

                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(LoadStrategyScreenCommands::new(&self.state)),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");
    
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
                        Some(&"IdentityTopUp") => Some(Message::IdentityTopUp),
                        Some(&"IdentityUpdate") => Some(Message::ExpectingInput(InputType::IdentityUpdate)),
                        Some(&"IdentityWithdrawal") => Some(Message::IdentityWithdrawal),
                        Some(&"ContractCreate") => Some(Message::ContractCreate),
                        Some(&"ContractUpdate") => Some(Message::ExpectingInput(InputType::ContractUpdate)),
                        Some(&"IdentityTransfer") => Some(Message::IdentityTransfer),
                        _ => None,
                    }
                },
                Message::Frequency(field, tpbr, cpb) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");

                    let current_name = &self.state.current_strategy;
                    let current_strategy = self.state.available_strategies.get_mut(&current_name.clone().unwrap()).unwrap();

                    match field.as_str() {
                        "operations" => {
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

                            let mut last_op = current_strategy.operations.pop().unwrap();
                            last_op.frequency = Frequency {
                                times_per_block_range: tpbr..tpbr+1,
                                chance_per_block: Some(cpb)
                            };

                            current_strategy.operations.push(last_op);

                            self.state.save();

                            self.app
                                .remount(
                                    ComponentId::Screen,
                                    Box::new(StrategyOperationsScreen::new(&self.state)),
                                    make_screen_subs(),
                                )
                                .expect("unable to remount screen");
                        },
                        "identities_inserts" => {
                            self.app
                                .mount(
                                    ComponentId::CommandPallet,
                                    Box::new(StrategyIdentityInsertsScreenCommands::new()),
                                    vec![],
                                )
                                .expect("unable to mount component");    
                            self.app
                                .active(&ComponentId::CommandPallet)
                                .expect("cannot set active");

                            current_strategy.identities_inserts = Frequency {
                                times_per_block_range: 1..tpbr,
                                chance_per_block: Some(cpb),
                            };

                            self.state.save();

                            self.app
                                .remount(
                                    ComponentId::Screen,
                                    Box::new(StrategyIdentityInsertsScreen::new(&self.state)),
                                    make_screen_subs(),
                                )
                                .expect("unable to remount screen");

                        },
                        _ => {
                            panic!("this is not a valid strategy struct field")
                        }
                    }
    
                    Some(Message::Redraw)
                },
                Message::DocumentOp(contract, doc_type, action) => {
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
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::Document(doc_op),
                        frequency: Frequency::default(),
                    });

                    self.state.save();

                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::IdentityTopUp => {
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::IdentityTopUp,
                        frequency: Frequency::default(),
                    });
                    
                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::IdentityWithdrawal => {
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::IdentityWithdrawal,
                        frequency: Frequency::default(),
                    });
                    
                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::IdentityTransfer => {
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::IdentityTransfer,
                        frequency: Frequency::default(),
                    });
                    
                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::IdentityUpdate(op, count) => {
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

                    let op = match op.as_str() {
                        "add" => IdentityUpdateOp::IdentityUpdateAddKeys(count),
                        "disable" => IdentityUpdateOp::IdentityUpdateDisableKey(count),
                        _ => panic!("not an IdentityUpdate variant")
                    };
                    
                    let mut op_vec = Vec::new();
                    op_vec.push(op.clone());
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::IdentityUpdate(op.clone()),
                        frequency: Frequency::default(),
                    });

                    self.state.save();

                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::RemoveOperation => {
                    let current_name = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_name).unwrap();
                    current_strategy.operations.pop();
                
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
                    self.state.available_strategies.insert("new_strategy".to_string(), default_strategy());
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

                        Some(Message::ExpectingInput(InputType::RenameStrategy))
                    } else { None }
                },
                Message::DeleteStrategy(index) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(LoadStrategyScreenCommands::new(&self.state)),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");

                    if let Some(key) = self.state.available_strategies.keys().nth(index).cloned() {
                        if self.state.current_strategy == Some(key.clone()) {
                            self.state.current_strategy = None
                        }
                        self.state.available_strategies.remove(&key);
                    }
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
                Message::RemoveIdentityInserts => {
                    let current_name = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_name).unwrap();
                    current_strategy.identities_inserts = Frequency::default();
                    current_strategy.strategy_description().insert("identities_inserts".to_string(), "".to_string());
                
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
                Message::StartIdentities(count, key_count) => {
                    self.app
                        .umount(&ComponentId::Input)
                        .expect("unable to umount component");
                    self.app
                        .mount(
                            ComponentId::CommandPallet,
                            Box::new(StrategyStartIdentitiesScreenCommands::new()),
                            vec![],
                        )
                        .expect("unable to mount component");
                    self.app
                        .active(&ComponentId::CommandPallet)
                        .expect("cannot set active");


                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();

                    let identities = create_identities_state_transitions(
                        count,
                        key_count,
                        &mut SimpleSigner::default(),
                        &mut StdRng::seed_from_u64(567),
                        PlatformVersion::latest(),
                    );

                    current_strategy.start_identities = identities;

                    self.state.save();

                    self.app
                        .remount(
                            ComponentId::Screen,
                            Box::new(StrategyStartIdentitiesScreen::new(&self.state)),
                            make_screen_subs(),
                        )
                        .expect("unable to remount screen");

                    Some(Message::Redraw)
                },
                Message::RemoveStartIdentities => {
                    let current_name = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_name).unwrap();
                    current_strategy.start_identities = vec![];
                
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
                Message::ContractCreate => {
                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();

                    let random_number1 = rand::thread_rng().gen_range(1..=50);
                    let random_number2 = rand::thread_rng().gen_range(1..=50);
                    let random_number3 = rand::thread_rng().gen::<i64>()-1000000;
                    
                    current_strategy.operations.push(Operation {
                        op_type: OperationType::ContractCreate(
                            RandomDocumentTypeParameters {
                                new_fields_optional_count_range: 1..random_number1,
                                new_fields_required_count_range: 1..random_number2,
                                new_indexes_count_range: 1..rand::thread_rng().gen_range(1..=(min(random_number1+random_number2, 10))),
                                field_weights: FieldTypeWeights {
                                    string_weight: rand::thread_rng().gen_range(1..=100),
                                    float_weight: rand::thread_rng().gen_range(1..=100),
                                    integer_weight: rand::thread_rng().gen_range(1..=100),
                                    date_weight: rand::thread_rng().gen_range(1..=100),
                                    boolean_weight: rand::thread_rng().gen_range(1..=100),
                                    byte_array_weight: rand::thread_rng().gen_range(1..=100),
                                },
                                field_bounds: FieldMinMaxBounds {
                                    string_min_len: 1..10,
                                    string_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    string_max_len: 10..63,
                                    string_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    integer_min: 1..10,
                                    integer_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    integer_max: 10..10000,
                                    integer_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    float_min: 0.1..10.0,
                                    float_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    float_max: 10.0..1000.0,
                                    float_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    date_min: random_number3,
                                    date_max: random_number3+1000000,
                                    byte_array_min_len: 1..10,
                                    byte_array_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    byte_array_max_len: 10..255,
                                    byte_array_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                },
                                keep_history_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                documents_mutable_chance: rand::thread_rng().gen_range(0.01..=1.0),
                            }, 
                            1..rand::thread_rng().gen::<u16>()),
                        frequency: Frequency::default(),
                    });
                    
                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
                Message::ContractUpdate(index) => {

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

                    let current_strategy_key = self.state.current_strategy.clone().unwrap();
                    let current_strategy = self.state.available_strategies.get_mut(&current_strategy_key).unwrap();
                    
                    let op = match index {
                        0 => { 
                            let random_number1 = rand::thread_rng().gen_range(1..=50);
                            let random_number2 = rand::thread_rng().gen_range(1..=50);
                            let random_number3 = rand::thread_rng().gen::<i64>()-1000000;

                            DataContractUpdateOp::DataContractNewDocumentTypes(
                                RandomDocumentTypeParameters {
                                    new_fields_optional_count_range: 1..random_number1,
                                    new_fields_required_count_range: 1..random_number2,
                                    new_indexes_count_range: 1..rand::thread_rng().gen_range(1..=(min(random_number1+random_number2, 10))),
                                    field_weights: FieldTypeWeights {
                                        string_weight: rand::thread_rng().gen_range(1..=100),
                                        float_weight: rand::thread_rng().gen_range(1..=100),
                                        integer_weight: rand::thread_rng().gen_range(1..=100),
                                        date_weight: rand::thread_rng().gen_range(1..=100),
                                        boolean_weight: rand::thread_rng().gen_range(1..=100),
                                        byte_array_weight: rand::thread_rng().gen_range(1..=100),
                                    },
                                    field_bounds: FieldMinMaxBounds {
                                        string_min_len: 1..10,
                                        string_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        string_max_len: 10..63,
                                        string_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        integer_min: 1..10,
                                        integer_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        integer_max: 10..10000,
                                        integer_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        float_min: 0.1..10.0,
                                        float_has_min_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        float_max: 10.0..1000.0,
                                        float_has_max_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        date_min: random_number3,
                                        date_max: random_number3+1000000,
                                        byte_array_min_len: 1..10,
                                        byte_array_has_min_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                        byte_array_max_len: 10..255,
                                        byte_array_has_max_len_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    },
                                    keep_history_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                    documents_mutable_chance: rand::thread_rng().gen_range(0.01..=1.0),
                                }
                            )
                        },
                        1 => DataContractUpdateOp::DataContractNewOptionalFields(1..20, 1..3),
                        _ => panic!("index out of bounds for DataContractUpdateOp")
                    };

                    current_strategy.operations.push(Operation {
                        op_type: OperationType::ContractUpdate(op),
                        frequency: Frequency::default(),
                    });
                    
                    Some(Message::ExpectingInput(InputType::Frequency("operations".to_string())))
                },
            }
        } else {
            None
        }
    }
}
