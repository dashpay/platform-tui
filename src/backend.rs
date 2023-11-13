//! Application backend.
//! This includes all logic unrelated to UI.

mod identities;
mod insight;
mod state;
mod strategies;
mod wallet;

use std::{
    fmt::Display,
    sync::{RwLock, RwLockReadGuard},
};

use rs_sdk::Sdk;
use serde::Serialize;
pub(crate) use state::AppState;
use strategy_tests::frequency::Frequency;
use tokio::sync::Mutex;

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String),
    SelectStrategy(String),
    StrategySetIdentityInserts {
        strategy_name: String,
        identity_inserts_frequency: Frequency,
    },
    StrategyStartIdentities {
        strategy_name: String,
        count: u16,
        key_count: u32,
    },
    /// For testing purposes
    None,
}

pub(crate) enum BackendEvent<'s> {
    IdentityLoaded,
    IdentityUnloaded,
    TaskCompleted(Task, Result<String, String>),
    TaskCompletedStateChange(Task, RwLockReadGuard<'s, AppState>),
    AppStateUpdated(RwLockReadGuard<'s, AppState>),
    None,
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
                let result = identities::fetch_identity_by_b58_id(&mut sdk, &base58_id).await;
                BackendEvent::TaskCompleted(task, result)
            }
            Task::SelectStrategy(strategy_name) => {
                self.app_state
                    .write()
                    .expect("lock is poisoned")
                    .selected_strategy = Some(strategy_name);
                BackendEvent::AppStateUpdated(self.app_state.read().expect("lock is poisoned"))
            }
            Task::StrategySetIdentityInserts {
                strategy_name,
                identity_inserts_frequency,
            } => {
                let state_updated = if let Some(strategy) = self
                    .app_state
                    .write()
                    .expect("lock is poisoned")
                    .available_strategies
                    .get_mut(&strategy_name)
                {
                    strategy.identities_inserts = identity_inserts_frequency;
                    true
                } else {
                    false
                };

                if state_updated {
                    BackendEvent::AppStateUpdated(self.app_state.read().expect("lock is poisoned"))
                } else {
                    BackendEvent::None
                }
            }
            Task::StrategyStartIdentities {
                ref strategy_name,
                count,
                key_count,
            } => {
                let state_updated = if let Some(strategy) = self
                    .app_state
                    .write()
                    .expect("lock is poisoned")
                    .available_strategies
                    .get_mut(strategy_name.as_str())
                {
                    strategies::set_start_identities(strategy, count, key_count);
                    true
                } else {
                    false
                };

                if state_updated {
                    BackendEvent::TaskCompletedStateChange(
                        task.clone(),
                        self.app_state.read().expect("lock is poisoned"),
                    )
                } else {
                    BackendEvent::None
                }
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
