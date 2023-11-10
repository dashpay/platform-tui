//! Application backend.
//! This includes all logic unrelated to UI.

mod identities;
mod insight;
mod state;
mod wallet;

use std::{
    collections::BTreeMap,
    fmt::Display,
    sync::{RwLock, RwLockReadGuard},
};

use rs_sdk::Sdk;
use serde::Serialize;
pub(crate) use state::AppState;
use strategy_tests::Strategy;
use tokio::sync::Mutex;

use self::identities::fetch_identity_by_b58_id;

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String),
    SelectStrategy(String),

    /// For testing purposes
    None,
}

pub(crate) enum BackendEvent {
    IdentityLoaded,
    IdentityUnloaded,
    TaskCompleted(Task, Result<String, String>),
    AppStateUpdated(AppStateUpdate),
}

pub(crate) enum AppStateUpdate {
    AvailableStrategies(BTreeMap<String, Strategy>),
    SelectedStrategy(String),
}

pub(crate) struct Backend {
    sdk: Mutex<Sdk>,
    app_state: RwLock<AppState>,
}

impl Backend {
    pub(crate) async fn new(sdk: Sdk) -> Self {
        Backend {
            sdk: Mutex::new(sdk),
            app_state: RwLock::new(AppState::load().await),
        }
    }

    pub(crate) fn state(&self) -> RwLockReadGuard<AppState> {
        self.app_state.read().expect("lock is poisoned")
    }

    pub(crate) async fn run_task(&self, task: Task) -> BackendEvent {
        match task {
            Task::FetchIdentityById(ref base58_id) => {
                let mut sdk = self.sdk.lock().await;
                let result = fetch_identity_by_b58_id(&mut sdk, &base58_id).await;
                BackendEvent::TaskCompleted(task, result)
            }
            Task::SelectStrategy(strategy) => {
                self.app_state
                    .write()
                    .expect("lock is poisoned")
                    .selected_strategy = Some(strategy.clone());
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(strategy))
            }
            Task::None => BackendEvent::TaskCompleted(task, Ok("".to_owned())),
        }
    }
}

fn stringify_result<T: Serialize, E: Display>(result: Result<T, E>) -> Result<String, String> {
    match result {
        Ok(data) => {
            Ok(toml::to_string_pretty(&data).unwrap_or("Cannot serialize as TOML".to_owned()))
        }
        Err(e) => Err(e.to_string()),
    }
}
