//! Withdrawals testing screen

use crate::{
    backend::{identities::IdentityTask, AppState, AppStateUpdate, BackendEvent, Task},
    ui::screen::{
        utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
        ScreenFeedback, ScreenToggleKey,
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

const COMMAND_KEYS: [ScreenCommandKey; 3] = [
    ScreenCommandKey::new("q", "Quit"),
    ScreenCommandKey::new("w", "Working withdrawal"),
    ScreenCommandKey::new("1", "Withdraw to wrong wallet"),
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
                code: Key::Char('w'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(WithdrawFromIdentityFormController::new())),
            Event::Key(KeyEvent {
                code: Key::Char('1'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Identity(IdentityTask::WithdrawToWrongAddress),
                block: true,
            },

            // Backend Event Handling
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::WithdrawFromIdentity(_)),
                execution_result,
                app_state_update: AppStateUpdate::LoadedIdentity(_),
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
                task: Task::Identity(IdentityTask::WithdrawToWrongAddress),
                execution_result,
                app_state_update: AppStateUpdate::LoadedIdentity(_),
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(IdentityTask::WithdrawToWrongAddress),
                execution_result,
            }) => {
                self.info = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }

            _ => ScreenFeedback::None,
        }
    }
}
