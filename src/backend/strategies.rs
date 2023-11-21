//! Strategies management backend module.

use std::collections::BTreeMap;

use dpp::version::PlatformVersion;
use rand::{rngs::StdRng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{
    frequency::Frequency, operations::Operation, transitions::create_identities_state_transitions,
    Strategy,
};
use tokio::sync::{Mutex, MutexGuard};

use super::{state::StrategiesMap, AppStateUpdate, BackendEvent, StrategyContractNames, Task};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum StrategyTask {
    CreateStrategy(String),
    SelectStrategy(String),
    DeleteStrategy(String),
    CloneStrategy(String),
    SetIdentityInserts {
        strategy_name: String,
        identity_inserts_frequency: Frequency,
    },
    SetStartIdentities {
        strategy_name: String,
        count: u16,
        key_count: u32,
    },
    AddOperation {
        strategy_name: String,
        operation: Operation,
    },
}

pub(crate) async fn run_strategy_task<'s>(
    available_strategies: &'s Mutex<StrategiesMap>,
    available_strategies_contract_names: &'s Mutex<BTreeMap<String, StrategyContractNames>>,
    selected_strategy: &'s Mutex<Option<String>>,
    task: StrategyTask,
) -> BackendEvent<'s> {
    match task {
        StrategyTask::CreateStrategy(strategy_name) => {
            let mut strategies_lock = available_strategies.lock().await;
            let mut contract_names_lock = available_strategies_contract_names.lock().await;

            strategies_lock.insert(
                strategy_name.clone(),
                Strategy {
                    contracts_with_updates: Default::default(),
                    operations: Default::default(),
                    start_identities: Default::default(),
                    identities_inserts: Default::default(),
                    signer: Default::default(),
                },
            );
            contract_names_lock.insert(strategy_name, Default::default());
            BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                strategies_lock,
                contract_names_lock,
            ))
        }
        StrategyTask::SelectStrategy(ref strategy_name) => {
            let mut selected_strategy_lock = selected_strategy.lock().await;
            let strategies_lock = available_strategies.lock().await;

            if strategies_lock.contains_key(strategy_name) {
                *selected_strategy_lock = Some(strategy_name.clone());
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::DeleteStrategy(strategy_name) => {
            let mut strategies_lock = available_strategies.lock().await;
            let mut contract_names_lock = available_strategies_contract_names.lock().await;
            let mut selected_strategy_lock = selected_strategy.lock().await;
        
            // Check if the strategy exists and remove it
            if strategies_lock.contains_key(&strategy_name) {
                strategies_lock.remove(&strategy_name);
                contract_names_lock.remove(&strategy_name);
        
                // If the deleted strategy was the selected one, unset the selected strategy
                if let Some(selected) = selected_strategy_lock.as_ref() {
                    if selected == &strategy_name {
                        *selected_strategy_lock = None;
                    }
                }
        
                BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                    strategies_lock,
                    contract_names_lock,
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::CloneStrategy(new_strategy_name) => {
            let strategies_lock = available_strategies.lock().await;
            let mut contract_names_lock = available_strategies_contract_names.lock().await;
            let selected_strategy_lock = selected_strategy.lock().await;
        
            if let Some(selected_strategy_name) = &*selected_strategy_lock {
                if let Some(strategy_to_clone) = strategies_lock.get(selected_strategy_name) {
                    let cloned_strategy = strategy_to_clone.clone();
                    drop(strategies_lock); // Release the lock before re-acquiring it as mutable
        
                    let mut strategies_lock = available_strategies.lock().await;
                    strategies_lock.insert(new_strategy_name.clone(), cloned_strategy);
                    contract_names_lock.insert(new_strategy_name, Default::default());
        
                    BackendEvent::AppStateUpdated(AppStateUpdate::Strategies(
                        strategies_lock,
                        contract_names_lock,
                    ))
                } else {
                    BackendEvent::None // Selected strategy does not exist
                }
            } else {
                BackendEvent::None // No strategy selected to clone
            }
        }        
        StrategyTask::SetIdentityInserts {
            strategy_name,
            identity_inserts_frequency,
        } => {
            let mut strategies_lock = available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(&strategy_name) {
                strategy.identities_inserts = identity_inserts_frequency;
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(&strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(&strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::SetStartIdentities {
            ref strategy_name,
            count,
            key_count,
        } => {
            let mut strategies_lock = available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                tokio::task::block_in_place(|| set_start_identities(strategy, count, key_count));
                BackendEvent::TaskCompletedStateChange {
                    task: Task::Strategy(task.clone()),
                    execution_result: Ok("Start identities set".to_owned()),
                    app_state_update: AppStateUpdate::SelectedStrategy(
                        strategy_name.clone(),
                        MutexGuard::map(strategies_lock, |strategies| {
                            strategies.get_mut(strategy_name).expect("strategy exists")
                        }),
                        MutexGuard::map(
                            available_strategies_contract_names.lock().await,
                            |names| names.get_mut(strategy_name).expect("inconsistent data"),
                        ),
                    ),
                }
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::AddOperation {
            ref strategy_name,
            ref operation,
        } => {
            let mut strategies_lock = available_strategies.lock().await;
            if let Some(strategy) = strategies_lock.get_mut(strategy_name) {
                strategy.operations.push(operation.clone());
                BackendEvent::AppStateUpdated(AppStateUpdate::SelectedStrategy(
                    strategy_name.clone(),
                    MutexGuard::map(strategies_lock, |strategies| {
                        strategies.get_mut(strategy_name).expect("strategy exists")
                    }),
                    MutexGuard::map(available_strategies_contract_names.lock().await, |names| {
                        names.get_mut(strategy_name).expect("inconsistent data")
                    }),
                ))
            } else {
                BackendEvent::None
            }
        }
    }
}

fn set_start_identities(strategy: &mut Strategy, count: u16, key_count: u32) {
    let identities = create_identities_state_transitions(
        count,
        key_count,
        &mut SimpleSigner::default(),
        &mut StdRng::seed_from_u64(567),
        PlatformVersion::latest(),
    );

    strategy.start_identities = identities;
}
