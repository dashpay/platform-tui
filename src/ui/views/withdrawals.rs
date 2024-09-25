//! Withdrawals testing screen

use crate::{
    backend::{identities::IdentityTask, AppState, AppStateUpdate, BackendEvent, Task},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, SelectInput},
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};
use clap::Id;
use dpp::prelude::{Identifier, Identity};
use std::collections::BTreeMap;
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::layout::Rect,
    Frame,
};

use super::wallet::WithdrawFromIdentityFormController;

const COMMAND_KEYS: [ScreenCommandKey; 4] = [
    ScreenCommandKey::new("q", "Quit"),
    ScreenCommandKey::new("1", "Working withdrawal"),
    ScreenCommandKey::new("2", "Withdraw to empty address"),
    ScreenCommandKey::new("3", "Select key type withdrawal"),
];

pub(crate) struct WithdrawalsScreenController {
    info: Info,
    identities_map: BTreeMap<Identifier, Identity>,
}

impl_builder!(WithdrawalsScreenController);

impl WithdrawalsScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let known_identities_lock = app_state.known_identities.lock().await;
        let info = Info::new_fixed("Withdrawals testing screen");

        Self {
            info,
            identities_map: known_identities_lock.clone(),
        }
    }
}

impl ScreenController for WithdrawalsScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area);
    }

    fn name(&self) -> &'static str {
        "Withdrawals Testing Screen"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            // Keys
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,
            Event::Key(KeyEvent {
                code: Key::Char('1'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(WithdrawFromIdentityFormController::new())),
            Event::Key(KeyEvent {
                code: Key::Char('2'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Identity(IdentityTask::WithdrawToNoAddress),
                block: true,
            },
            Event::Key(KeyEvent {
                code: Key::Char('3'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(SelectKeyTypeWithdrawalFormController::new())),

            // Backend Event Handling
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::WithdrawFromIdentity(_)),
                execution_result,
                app_state_update: AppStateUpdate::WithdrewFromIdentityToAddress(_),
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(IdentityTask::WithdrawFromIdentity(_)),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::WithdrawToNoAddress),
                execution_result,
                app_state_update: AppStateUpdate::WithdrewFromIdentityToAddress(_),
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(IdentityTask::WithdrawToNoAddress),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::SelectKeyTypeWithdrawal(_)),
                execution_result,
                app_state_update: AppStateUpdate::WithdrewFromIdentityToAddress(_),
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(IdentityTask::SelectKeyTypeWithdrawal(_)),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }

            _ => ScreenFeedback::None,
        }
    }
}

struct SelectKeyTypeWithdrawalFormController {
    input: SelectInput<String>,
}

impl SelectKeyTypeWithdrawalFormController {
    fn new() -> Self {
        Self {
            input: SelectInput::new(vec![
                "ECDSA_SECP256K1".to_string(),
                "BLS12_381".to_string(),
                "ECDSA_HASH160".to_string(),
                "BIP13_SCRIPT_HASH".to_string(),
                "EDDSA_25519_HASH160".to_string(),
            ]),
        }
    }
}

impl FormController for SelectKeyTypeWithdrawalFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(key_type) => FormStatus::Done {
                task: Task::Identity(IdentityTask::SelectKeyTypeWithdrawal(key_type)),
                block: false,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Select key type withdrawal"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Key type"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
