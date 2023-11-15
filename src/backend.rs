//! Application backend.
//! This includes all logic unrelated to UI.

mod contracts;
mod identities;
mod insight;
mod state;
mod strategies;
mod wallet;

use std::{
    fmt::Display,
    ops::DerefMut,
    sync::{RwLock, RwLockReadGuard},
};

use dash_platform_sdk::Sdk;
use serde::Serialize;
pub(crate) use state::AppState;
use tokio::sync::Mutex;

pub(crate) use self::{
    contracts::ContractTask,
    strategies::StrategyTask,
    wallet::{Wallet, WalletTask},
};

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String),
    Strategy(StrategyTask),
    Wallet(WalletTask),
    Contract(ContractTask),
}

pub(crate) enum BackendEvent<'s> {
    TaskCompleted(Task, Result<String, String>),
    TaskCompletedStateChange(Task, Result<String, String>, RwLockReadGuard<'s, AppState>),
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
            Task::Strategy(strategy_task) => {
                strategies::run_strategy_task(&self.app_state, strategy_task)
            }
            Task::Wallet(wallet_task) => {
                wallet::run_wallet_task(&self.app_state, wallet_task).await
            }
            Task::Contract(contract_task) => {
                contracts::run_contract_task(
                    self.sdk.lock().await.deref_mut(),
                    &self.app_state,
                    contract_task,
                )
                .await
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        if let Ok(state) = self.app_state.read() {
            state.save();
        }
    }
}

fn stringify_result<T: Serialize, E: Display>(result: Result<T, E>) -> Result<String, String> {
    match result {
        Ok(data) => Ok(as_toml(data)),
        Err(e) => Err(e.to_string()),
    }
}

fn as_toml<T: Serialize>(value: T) -> String {
    toml::to_string_pretty(&value).unwrap_or("Cannot serialize as TOML".to_owned())
}
