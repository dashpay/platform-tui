//! Application backend.
//! This includes all logic unrelated to UI.

mod contracts;
mod error;
pub(crate) mod identities;
pub(crate) mod info_display;
mod insight;
mod state;
mod strategies;
mod wallet;

use std::{
    collections::BTreeMap,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use dash_platform_sdk::Sdk;
use dpp::{identity::accessors::IdentityGettersV0, prelude::Identity};
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
use crate::backend::identities::IdentityTask;

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String, bool),
    Strategy(StrategyTask),
    Wallet(WalletTask),
    Identity(IdentityTask),
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
    IdentityRegistrationProgressed, // TODO provide state update details
    LoadedIdentity(MappedMutexGuard<'s, Identity>),
    UpdatedBalance(u64),
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
            Task::FetchIdentityById(ref base58_id, add_to_known_identities) => {
                let mut sdk = self.sdk.lock().await;
                let execution_result =
                    identities::fetch_identity_by_b58_id(&mut sdk, &base58_id).await;
                if add_to_known_identities {
                    if let Ok((Some(identity), _)) = &execution_result {
                        let mut loaded_identities = self.app_state.known_identities.lock().await;
                        loaded_identities.insert(identity.id(), identity.clone());
                    }
                }

                let execution_info_result = execution_result.map(|(_, result_info)| result_info);

                BackendEvent::TaskCompleted {
                    task,
                    execution_result: execution_info_result,
                }
            }
            Task::Strategy(strategy_task) => {
                strategies::run_strategy_task(
                    &self.app_state.available_strategies,
                    &self.app_state.available_strategies_contract_names,
                    &self.app_state.selected_strategy,
                    &self.app_state.known_contracts,
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
            Task::Identity(identity_task) => {
                self.app_state
                    .run_identity_task(self.sdk.lock().await.deref(), identity_task)
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
        Ok(data) => Ok(as_toml(&data)),
        Err(e) => Err(e.to_string()),
    }
}

fn stringify_result_keep_item<T: Serialize, E: Display>(
    result: Result<T, E>,
) -> Result<(T, String), String> {
    match result {
        Ok(data) => {
            let toml = as_toml(&data);
            Ok((data, toml))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn as_toml<T: Serialize>(value: &T) -> String {
    toml::to_string_pretty(&value).unwrap_or("Cannot serialize as TOML".to_owned())
}
