//! Screens and forms related to wallet management.

use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::main::MainScreenController;
use crate::{
    backend::{AppState, BackendEvent, Task, Wallet, WalletTask},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, TextInput},
        screen::{
            widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback,
            ScreenToggleKey,
        },
    },
    Event,
};

const NO_WALLET_COMMANDS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("a", "Add by private key"),
];

const COMMANDS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("r", "Refresh utxos and balance"),
];

pub(crate) struct WalletScreenController {
    info: Info,
    wallet_loaded: bool,
}

impl WalletScreenController {
    pub(crate) fn new(app_state: &AppState) -> Self {
        let (info, wallet_loaded) = if let Some(wallet) = &app_state.loaded_wallet {
            (Info::new_fixed(&display_wallet(wallet)), true)
        } else {
            (
                Info::new_fixed("Wallet management commands\n\nNo wallet loaded yet"),
                false,
            )
        };

        WalletScreenController {
            info,
            wallet_loaded,
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
            COMMANDS.as_ref()
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
            }) => {
                ScreenFeedback::PreviousScreen(Box::new(|_| Box::new(MainScreenController::new())))
            }
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

            Event::Backend(
                BackendEvent::AppStateUpdated(app_state)
                | BackendEvent::TaskCompletedStateChange(_, app_state),
            ) => {
                if let Some(wallet) = &app_state.loaded_wallet {
                    self.info = Info::new_fixed(&display_wallet(wallet));
                    self.wallet_loaded = true;
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
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
