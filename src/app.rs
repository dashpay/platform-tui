//! Application logic module, includes model and screen ids.

mod contract;
pub(crate) mod error;
mod identity;
pub(crate) mod state;
mod wallet;

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

use crate::components::*;

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
    RegisterIdentity,
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
                }
                Message::RegisterIdentity => {
                    // first we need to make the transaction
                    //                    self.state.register_identity()
                    None
                }
            }
        } else {
            None
        }
    }
}
