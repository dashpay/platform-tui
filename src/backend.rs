//! Application backend.
//! This includes all logic unrelated to UI.

mod contracts;
mod identities;
mod insight;
mod state;
mod strategies;
mod wallet;
mod info_display;

use std::{collections::BTreeMap, fmt::Display, ops::DerefMut};

use dash_platform_sdk::Sdk;
use serde::Serialize;
pub(crate) use state::AppState;
use strategy_tests::Strategy;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

use self::state::{KnownContractsMap, StrategiesMap};
pub(crate) use self::{
    contracts::ContractTask,
    state::StrategyContractNames,
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
    TaskCompleted {
        task: Task,
        execution_result: Result<String, String>,
    },
    TaskCompletedStateChange {
        task: Task,
        execution_result: Result<String, String>,
        app_state_update: AppStateUpdate<'s>,
    },
    AppStateUpdated(AppStateUpdate<'s>),
    None,
}

pub(crate) enum AppStateUpdate<'s> {
    KnownContracts(MutexGuard<'s, KnownContractsMap>),
    LoadedWallet(MappedMutexGuard<'s, Wallet>),
    Strategies(
        MutexGuard<'s, StrategiesMap>,
        MutexGuard<'s, BTreeMap<String, StrategyContractNames>>,
    ),
    SelectedStrategy(
        String,
        MappedMutexGuard<'s, Strategy>,
        MappedMutexGuard<'s, StrategyContractNames>,
    ),
}

pub(crate) struct Backend {
    sdk: Mutex<Sdk>,
    app_state: AppState,
}

impl Backend {
    pub(crate) async fn new(sdk: Sdk) -> Self {
        Backend {
            sdk: Mutex::new(sdk),
            app_state: AppState::load().await,
        }
    }

    pub(crate) fn state(&self) -> &AppState {
        &self.app_state
    }

    pub(crate) async fn run_task(&self, task: Task) -> BackendEvent {
        match task {
            Task::FetchIdentityById(ref base58_id) => {
                let mut sdk = self.sdk.lock().await;
                let execution_result =
                    identities::fetch_identity_by_b58_id(&mut sdk, &base58_id).await;
                BackendEvent::TaskCompleted {
                    task,
                    execution_result,
                }
            }
            Task::Strategy(strategy_task) => {
                strategies::run_strategy_task(
                    &self.app_state.available_strategies,
                    &self.app_state.available_strategies_contract_names,
                    &self.app_state.selected_strategy,
                    strategy_task,
                )
                .await
            }
            Task::Wallet(wallet_task) => {
                wallet::run_wallet_task(&self.app_state.loaded_wallet, wallet_task).await
            }
            Task::Contract(contract_task) => {
                contracts::run_contract_task(
                    self.sdk.lock().await.deref_mut(),
                    &self.app_state.known_contracts,
                    contract_task,
                )
                .await
            }
        }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.app_state.save()
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
