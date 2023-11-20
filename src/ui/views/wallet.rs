//! Screens and forms related to wallet management.

use dpp::prelude::Identity;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::main::MainScreenController;
use crate::backend::identities::IdentityTask;
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, Task, Wallet, WalletTask},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, TextInput},
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const NO_WALLET_COMMANDS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("a", "Add by private key"),
];

const WALLET_BUT_NO_IDENTITY_COMMANDS: [ScreenCommandKey; 4] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("r", "Refresh utxos and balance"),
    ScreenCommandKey::new("c", "Copy Address"),
    ScreenCommandKey::new("i", "Register Identity"),
];

const COMMANDS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("r", "Refresh utxos and balance"),
    ScreenCommandKey::new("c", "Copy Address"),
];

pub(crate) struct WalletScreenController {
    info: Info,
    wallet_loaded: bool,
    identity_loaded: bool,
}

impl_builder!(WalletScreenController);

impl WalletScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let (info, wallet_loaded, identity_loaded) =
            if let Some(wallet) = app_state.loaded_wallet.lock().await.as_ref() {
                if let Some(identity) = app_state.loaded_identity.lock().await.as_ref() {
                    (
                        Info::new_fixed(&display_wallet_and_identity(wallet, identity)),
                        true,
                        true,
                    )
                } else {
                    (Info::new_fixed(&display_wallet(wallet)), true, false)
                }
            } else {
                (
                    Info::new_fixed("Wallet management commands\n\nNo wallet loaded yet"),
                    false,
                    false,
                )
            };

        WalletScreenController {
            info,
            wallet_loaded,
            identity_loaded,
        }
    }
}

impl ScreenController for WalletScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }

    fn name(&self) -> &'static str {
        "Wallet"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        if self.wallet_loaded {
            if self.identity_loaded {
                COMMANDS.as_ref()
            } else {
                WALLET_BUT_NO_IDENTITY_COMMANDS.as_ref()
            }
        } else {
            NO_WALLET_COMMANDS.as_ref()
        }
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen(MainScreenController::builder()),
            Event::Key(KeyEvent {
                code: Key::Char('a'),
                modifiers: KeyModifiers::NONE,
            }) if !self.wallet_loaded => {
                ScreenFeedback::Form(Box::new(AddWalletPrivateKeyFormController::new()))
            }
            Event::Key(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded => ScreenFeedback::Task {
                task: Task::Wallet(WalletTask::Refresh),
                block: true,
            },
            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded && !self.identity_loaded => ScreenFeedback::Task {
                task: Task::Identity(IdentityTask::RegisterIdentity(1000000000)),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded => ScreenFeedback::Task {
                task: Task::Wallet(WalletTask::CopyAddress),
                block: true,
            },

            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::LoadedWallet(wallet))
                | BackendEvent::TaskCompletedStateChange {
                    app_state_update: AppStateUpdate::LoadedWallet(wallet),
                    ..
                },
            ) => {
                self.info = Info::new_fixed(&display_wallet(&wallet));
                self.wallet_loaded = true;
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

struct AddWalletPrivateKeyFormController {
    input: TextInput,
}

impl AddWalletPrivateKeyFormController {
    fn new() -> Self {
        AddWalletPrivateKeyFormController {
            input: TextInput::new("SHA256 hex"),
        }
    }
}

impl FormController for AddWalletPrivateKeyFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(private_key) => FormStatus::Done {
                task: Task::Wallet(WalletTask::AddByPrivateKey(private_key)),
                block: false,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
        }
    }

    fn form_name(&self) -> &'static str {
        "Add wallet with private key"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Private key"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

fn display_wallet(wallet: &Wallet) -> String {
    wallet.description()
}

fn display_wallet_and_identity(wallet: &Wallet, identity: &Identity) -> String {
    wallet.description()
}
