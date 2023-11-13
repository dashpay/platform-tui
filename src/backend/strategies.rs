//! Strategies management backend module.

use std::sync::RwLock;

use dpp::version::PlatformVersion;
use rand::{rngs::StdRng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use strategy_tests::{
    frequency::Frequency, operations::Operation, transitions::create_identities_state_transitions,
    Strategy,
};

use super::{AppState, BackendEvent, Task};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum StrategyTask {
    SelectStrategy(String),
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

pub(crate) fn run_strategy_task(app_state: &RwLock<AppState>, task: StrategyTask) -> BackendEvent {
    match task {
        StrategyTask::SelectStrategy(strategy_name) => {
            app_state
                .write()
                .expect("lock is poisoned")
                .selected_strategy = Some(strategy_name);
            BackendEvent::AppStateUpdated(app_state.read().expect("lock is poisoned"))
        }
        StrategyTask::SetIdentityInserts {
            strategy_name,
            identity_inserts_frequency,
        } => {
            let state_updated = if let Some(strategy) = app_state
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
                BackendEvent::AppStateUpdated(app_state.read().expect("lock is poisoned"))
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::SetStartIdentities {
            ref strategy_name,
            count,
            key_count,
        } => {
            let state_updated = if let Some(strategy) = app_state
                .write()
                .expect("lock is poisoned")
                .available_strategies
                .get_mut(strategy_name.as_str())
            {
                set_start_identities(strategy, count, key_count);
                true
            } else {
                false
            };

            if state_updated {
                BackendEvent::TaskCompletedStateChange(
                    Task::Strategy(task.clone()),
                    app_state.read().expect("lock is poisoned"),
                )
            } else {
                BackendEvent::None
            }
        }
        StrategyTask::AddOperation {
            ref strategy_name,
            ref operation,
        } => {
            let state_updated = if let Some(strategy) = app_state
                .write()
                .expect("lock is poisoned")
                .available_strategies
                .get_mut(strategy_name.as_str())
            {
                strategy.operations.push(operation.clone());
                true
            } else {
                false
            };

            if state_updated {
                BackendEvent::TaskCompletedStateChange(
                    Task::Strategy(task.clone()),
                    app_state.read().expect("lock is poisoned"),
                )
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
