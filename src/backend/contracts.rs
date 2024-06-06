//! Contracts backend.
use dash_sdk::{platform::Fetch, Sdk};
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    platform_value::string_encoding::Encoding,
    prelude::{DataContract, Identifier},
    system_data_contracts::{dashpay_contract, dpns_contract},
};
use tokio::sync::Mutex;

use super::{
    as_json_string, as_toml, state::KnownContractsMap, AppStateUpdate, BackendEvent, Task,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ContractTask {
    FetchDashpayContract,
    FetchDPNSContract,
    RemoveContract(String),
    FetchContract(String),
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
            match DataContract::fetch(&sdk, Into::<Identifier>::into(dashpay_contract::ID_BYTES))
                .await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_json_string(&data_contract);
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
            match DataContract::fetch(&sdk, Into::<Identifier>::into(dpns_contract::ID_BYTES)).await
            {
                Ok(Some(data_contract)) => {
                    let contract_str = as_json_string(&data_contract);
                    let mut contracts_lock = known_contracts.lock().await;
                    contracts_lock.insert(
                        data_contract.id().to_string(Encoding::Base58),
                        data_contract,
                    );

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
        ContractTask::RemoveContract(ref contract_name) => {
            let mut contracts_lock = known_contracts.lock().await;
            contracts_lock.remove(contract_name);
            BackendEvent::TaskCompletedStateChange {
                task: Task::Contract(task),
                execution_result: Ok("Contract removed".into()),
                app_state_update: AppStateUpdate::KnownContracts(contracts_lock),
            }
        }
        ContractTask::FetchContract(ref contract_id_string) => {
            let id = Identifier::from_string(&contract_id_string, Encoding::Base58)
                .expect("Expected to convert contract_id_string to Identifier");
            match DataContract::fetch(&sdk, id).await {
                Ok(Some(data_contract)) => {
                    let contract_str = as_json_string(&data_contract);
                    let mut contracts_lock = known_contracts.lock().await;
                    contracts_lock.insert(contract_id_string.to_string(), data_contract);

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
