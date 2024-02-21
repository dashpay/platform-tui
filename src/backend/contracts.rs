//! Contracts backend.

use rs_sdk::{platform::Fetch, Sdk};
use dpp::{
    prelude::{DataContract, Identifier},
    system_data_contracts::{dashpay_contract, dpns_contract},
};
use tokio::sync::Mutex;

use super::{as_toml, state::KnownContractsMap, AppStateUpdate, BackendEvent, Task};

#[derive(Clone, PartialEq)]
pub(crate) enum ContractTask {
    FetchDashpayContract,
    FetchDPNSContract,
}

const DASHPAY_CONTRACT_NAME: &str = "dashpay";
const DPNS_CONTRACT_NAME: &str = "dpns";

pub(super) async fn run_contract_task<'s>(
    sdk: &Sdk,
    known_contracts: &'s Mutex<KnownContractsMap>,
    task: ContractTask,
) -> BackendEvent<'s> {
    match task {
        ContractTask::FetchDashpayContract => {
            match DataContract::fetch(sdk, Into::<Identifier>::into(dashpay_contract::ID_BYTES))
                .await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_toml(&data_contract);
                    let mut contracts_lock = known_contracts.lock().await;
                    contracts_lock.insert(DASHPAY_CONTRACT_NAME.to_owned(), data_contract);

                    BackendEvent::TaskCompletedStateChange {
                        task: Task::Contract(task),
                        execution_result: Ok(contract_str.into()),
                        app_state_update: AppStateUpdate::KnownContracts(contracts_lock),
                    }
                }
                Ok(None) => BackendEvent::TaskCompleted {
                    task: Task::Contract(task),
                    execution_result: Ok("No contract".into()),
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::Contract(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
        ContractTask::FetchDPNSContract => {
            match DataContract::fetch(sdk, Into::<Identifier>::into(dpns_contract::ID_BYTES)).await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_toml(&data_contract);
                    let mut contracts_lock = known_contracts.lock().await;
                    contracts_lock.insert(DPNS_CONTRACT_NAME.to_owned(), data_contract);

                    BackendEvent::TaskCompletedStateChange {
                        task: Task::Contract(task),
                        execution_result: Ok(contract_str.into()),
                        app_state_update: AppStateUpdate::KnownContracts(contracts_lock),
                    }
                }
                Ok(None) => BackendEvent::TaskCompleted {
                    task: Task::Contract(task),
                    execution_result: Ok("No contract".into()),
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::Contract(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
    }
}
