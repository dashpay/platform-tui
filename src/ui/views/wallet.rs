//! Screens and forms related to wallet management.

use dpp::prelude::Identity;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::main::MainScreenController;
use crate::{
    backend::{
        identities::IdentityTask, AppState, AppStateUpdate, BackendEvent, Task, Wallet, WalletTask,
    },
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, TextInput},
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const WALLET_LOADED_COMMANDS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("r", "Refresh utxos and balance"),
    ScreenCommandKey::new("c", "Copy Address"),
];

const IDENTITY_LOADED_COMMANDS: [ScreenCommandKey; 1] =
    [ScreenCommandKey::new("t", "Identity top up")];

#[memoize::memoize]
fn join_commands(
    wallet_loaded: bool,
    identity_loaded: bool,
    identity_in_progress: bool,
) -> &'static [ScreenCommandKey] {
    let mut commands = vec![ScreenCommandKey::new("q", "Back to Main")];

    if wallet_loaded {
        commands.extend_from_slice(&WALLET_LOADED_COMMANDS);
    } else {
        commands.push(ScreenCommandKey::new("a", "Add wallet by private key"));
    }

    if identity_loaded {
        commands.extend_from_slice(&IDENTITY_LOADED_COMMANDS);
    } else {
        if identity_in_progress {
            commands.push(ScreenCommandKey::new("i", "Continue identity registration"));
        } else {
            commands.push(ScreenCommandKey::new("i", "Register identity"));
        }
    }

    commands.leak()
}

pub(crate) struct WalletScreenController {
    info: Info,
    wallet_loaded: bool,
    identity_loaded: bool,
    identity_in_progress: bool,
}

impl_builder!(WalletScreenController);

struct RegisterIdentityFormController {
    input: TextInput<u64>,
}

impl RegisterIdentityFormController {
    fn new() -> Self {
        RegisterIdentityFormController {
            input: TextInput::new("Quantity (unsigned integer)"),
        }
    }
}

impl FormController for RegisterIdentityFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(count) => FormStatus::Done {
                task: Task::Identity(IdentityTask::RegisterIdentity(count)),
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity registration"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Identities quantity"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

impl WalletScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let (info, wallet_loaded, identity_loaded, identity_in_progress) =
            if let Some(wallet) = app_state.loaded_wallet.lock().await.as_ref() {
                if let Some(identity) = app_state.loaded_identity.lock().await.as_ref() {
                    (
                        Info::new_fixed(&display_wallet_and_identity(wallet, identity)),
                        true,
                        true,
                        false,
                    )
                } else {
                    let identity_in_progress = app_state
                        .identity_asset_lock_private_key_in_creation
                        .lock()
                        .await
                        .is_some();
                    (
                        Info::new_fixed(&display_wallet(wallet)),
                        true,
                        false,
                        identity_in_progress,
                    )
                }
            } else {
                (
                    Info::new_fixed("Wallet management commands\n\nNo wallet loaded yet"),
                    false,
                    false,
                    false,
                )
            };

        WalletScreenController {
            info,
            wallet_loaded,
            identity_loaded,
            identity_in_progress,
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
        join_commands(
            self.wallet_loaded,
            self.identity_loaded,
            self.identity_in_progress,
        )
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
            }) => ScreenFeedback::Form(Box::new(RegisterIdentityFormController::new())),

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

            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::RegisterIdentity(_)),
                execution_result,
                app_state_update: AppStateUpdate::LoadedIdentity(_identity),
            }) => {
                self.info = Info::new_from_result(execution_result);
                self.identity_loaded = true;
                self.identity_in_progress = false;
                ScreenFeedback::Redraw
            }

            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::RegisterIdentity(_)),
                execution_result,
                app_state_update: AppStateUpdate::IdentityRegistrationProgressed,
            }) => {
                self.info = Info::new_from_result(execution_result);
                self.identity_loaded = false;
                self.identity_in_progress = true;
                ScreenFeedback::Redraw
            }

            // TODO identity register in progress state change
            Event::Backend(BackendEvent::TaskCompleted {
                execution_result, ..
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

struct AddWalletPrivateKeyFormController {
    input: TextInput<String>, // TODO: provide parser to always have a typesafe valid output
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
            InputStatus::Exit => FormStatus::Exit,
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
