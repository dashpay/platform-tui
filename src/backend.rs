//! Application backend.
//! This includes all logic unrelated to UI.

mod contracts;
pub(crate) mod documents;
mod error;
pub(crate) mod identities;
pub(crate) mod insight;
pub(crate) mod platform_info;
mod state;
mod strategies;
mod wallet;

use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    sync::atomic::{AtomicUsize, Ordering},
};

use dash_platform_sdk::Sdk;
use dpp::{
    document::Document,
    identity::accessors::IdentityGettersV0,
    prelude::{Identifier, Identity},
};
use serde::Serialize;
pub(crate) use state::AppState;
use strategy_tests::Strategy;
use tokio::sync::{MappedMutexGuard, MutexGuard};

use self::state::{KnownContractsMap, StrategiesMap};
pub(crate) use self::{
    contracts::ContractTask,
    state::StrategyContractNames,
    strategies::StrategyTask,
    wallet::{Wallet, WalletTask},
};
use crate::{
    backend::{
        documents::DocumentTask, identities::IdentityTask, insight::InsightAPIClient,
        platform_info::PlatformInfoTask,
    },
    config::Config,
};

pub(crate) type TaskId = usize;

/// Unit of work for the backend.
/// UI shall not execute any actions unrelated to rendering directly, to keep
/// things decoupled and for future UI/UX improvements it returns a [Task]
/// instead.
pub(crate) struct Task {
    kind: TaskKind,
    id: TaskId,
}

impl Task {
    pub(crate) fn new(kind: TaskKind) -> Self {
        static TASK_ID: AtomicUsize = AtomicUsize::new(0);
        let id = TASK_ID.fetch_add(1, Ordering::Relaxed);

        Task { kind, id }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }
}

pub(crate) enum TaskKind {
    FetchIdentityById(String, bool),
    PlatformInfo(PlatformInfoTask),
    Strategy(StrategyTask),
    Wallet(WalletTask),
    Identity(IdentityTask),
    Contract(ContractTask),
    Document(DocumentTask),
}

/// A positive task execution result.
/// Occasionally it's desired to represent data on UI in a structured way, in
/// that case specific variants are used.
pub(crate) enum CompletedTaskPayload {
    None,
    Documents(BTreeMap<Identifier, Option<Document>>),
    Document(Document),
    String(String),
}

impl From<String> for CompletedTaskPayload {
    fn from(value: String) -> Self {
        CompletedTaskPayload::String(value)
    }
}

impl From<&str> for CompletedTaskPayload {
    fn from(value: &str) -> Self {
        CompletedTaskPayload::String(value.to_owned())
    }
}

impl Display for CompletedTaskPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompletedTaskPayload::String(s) => write!(f, "{}", s),
            _ => write!(f, "Executed successfully"),
        }
    }
}

/// Any update coming from backend that UI may or may not react to.
pub(crate) struct BackendEvent<'s> {
    pub task_execution_result: Result<CompletedTaskPayload, String>,
    pub task_id: TaskId,
    pub state_update: AppStateUpdate<'s>,
}

#[derive(Default)]
pub(crate) struct BackendEventBuilder<'s> {
    task_execution_result: Option<Result<CompletedTaskPayload, String>>,
    state_update: Option<AppStateUpdate<'s>>,
}

impl<'s> BackendEventBuilder<'s> {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn with_execution_result(
        mut self,
        execution_result: Result<CompletedTaskPayload, String>,
    ) -> Self {
        self.task_execution_result = Some(execution_result);
        self
    }

    pub(crate) fn with_state_update(mut self, state_update: AppStateUpdate<'s>) -> Self {
        self.state_update = Some(state_update);
        self
    }

    pub(crate) fn build(self, task_id: TaskId) -> BackendEvent<'s> {
        BackendEvent {
            task_execution_result: self
                .task_execution_result
                .unwrap_or(Ok(CompletedTaskPayload::None)),
            task_id,
            state_update: self.state_update.unwrap_or(AppStateUpdate::None),
        }
    }
}

pub(crate) struct TaskResult {
    pub task_id: TaskId,
    pub execution_result: Result<CompletedTaskPayload, String>,
}

/// Backend state update data on a specific field.
/// A screen implementation may handle specific updates to deliver a responsive
/// UI.
pub(crate) enum AppStateUpdate<'s> {
    None,
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
    FailedToRefreshIdentity,
}

/// Application state, dependencies are task execution logic around it.
pub(crate) struct Backend<'a> {
    sdk: &'a Sdk,
    app_state: AppState,
    insight: InsightAPIClient,
    config: Config,
}

impl<'a> Backend<'a> {
    pub(crate) async fn new(
        sdk: &'a Sdk,
        insight: InsightAPIClient,
        config: Config,
    ) -> Backend<'a> {
        Backend {
            sdk,
            app_state: AppState::load(&insight, &config).await,
            insight,
            config,
        }
    }

    pub(crate) fn state(&self) -> &AppState {
        &self.app_state
    }

    pub(crate) async fn run_task(&self, Task { kind, id }: Task) -> BackendEvent {
        self.run_task_inner(kind).await.build(id)
    }

    async fn run_task_inner(&self, task: TaskKind) -> BackendEventBuilder {
        match task {
            TaskKind::FetchIdentityById(ref base58_id, add_to_known_identities) => {
                let execution_result =
                    identities::fetch_identity_by_b58_id(self.sdk, base58_id).await;
                if add_to_known_identities {
                    if let Ok((Some(identity), _)) = &execution_result {
                        let mut loaded_identities = self.app_state.known_identities.lock().await;
                        loaded_identities.insert(identity.id(), identity.clone());
                    }
                }

                let execution_info_result = execution_result
                    .map(|(_, result_info)| CompletedTaskPayload::String(result_info));
                BackendEventBuilder::default().with_execution_result(execution_info_result)
            }
            TaskKind::Strategy(strategy_task) => {
                strategies::run_strategy_task(
                    &self.app_state.available_strategies,
                    &self.app_state.available_strategies_contract_names,
                    &self.app_state.selected_strategy,
                    &self.app_state.known_contracts,
                    strategy_task,
                )
                .await
            }
            TaskKind::Wallet(wallet_task) => {
                wallet::run_wallet_task(&self.app_state.loaded_wallet, wallet_task, &self.insight)
                    .await
            }
            TaskKind::Contract(contract_task) => {
                contracts::run_contract_task(
                    self.sdk,
                    &self.app_state.known_contracts,
                    contract_task,
                )
                .await
            }
            TaskKind::Identity(identity_task) => {
                self.app_state
                    .run_identity_task(self.sdk, identity_task)
                    .await
            }
            TaskKind::Document(document_task) => {
                self.app_state
                    .run_document_task(self.sdk, document_task)
                    .await
            }
            TaskKind::PlatformInfo(platform_info_task) => {
                platform_info::run_platform_task(self.sdk, platform_info_task).await
            }
        }
    }
}

impl Drop for Backend<'_> {
    fn drop(&mut self) {
        self.app_state.save(&self.config)
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
