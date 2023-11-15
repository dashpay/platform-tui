//! Contracts backend.

use std::sync::RwLock;

use dash_platform_sdk::{platform::Fetch, Sdk};
use dpp::{
    prelude::{DataContract, Identifier},
    system_data_contracts::{dashpay_contract, dpns_contract},
};

use super::{as_toml, stringify_result, AppState, BackendEvent, Task};

#[derive(Clone, PartialEq)]
pub(crate) enum ContractTask {
    FetchDashpayContract,
    FetchDPNSContract,
}

const DASHPAY_CONTRACT_NAME: &str = "dashpay";
const DPNS_CONTRACT_NAME: &str = "dpns";

pub(super) async fn run_contract_task<'s>(
    sdk: &mut Sdk,
    app_state: &'s RwLock<AppState>,
    task: ContractTask,
) -> BackendEvent<'s> {
    match task {
        ContractTask::FetchDashpayContract => {
            match DataContract::fetch(sdk, Into::<Identifier>::into(dashpay_contract::ID_BYTES))
                .await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_toml(&data_contract);
                    app_state
                        .write()
                        .expect("lock is poisoned")
                        .known_contracts
                        .insert(DASHPAY_CONTRACT_NAME.to_owned(), data_contract);
                    BackendEvent::TaskCompletedStateChange(
                        Task::Contract(task),
                        Ok(contract_str),
                        app_state.read().expect("lock is poisoned"),
                    )
                }
                result => {
                    BackendEvent::TaskCompleted(Task::Contract(task), stringify_result(result))
                }
            }
        }
        ContractTask::FetchDPNSContract => {
            match DataContract::fetch(sdk, Into::<Identifier>::into(dpns_contract::ID_BYTES)).await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_toml(&data_contract);
                    app_state
                        .write()
                        .expect("lock is poisoned")
                        .known_contracts
                        .insert(DPNS_CONTRACT_NAME.to_owned(), data_contract);
                    BackendEvent::TaskCompletedStateChange(
                        Task::Contract(task),
                        Ok(contract_str),
                        app_state.read().expect("lock is poisoned"),
                    )
                }
                result => {
                    BackendEvent::TaskCompleted(Task::Contract(task), stringify_result(result))
                }
            }
        }
    }
}
