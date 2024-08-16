//! Contracts backend.
use std::{collections::BTreeMap, sync::Arc};

use dapi_grpc::platform::v0::{get_documents_request, GetDocumentsRequest};
use dash_sdk::{
    platform::{DocumentQuery, Fetch, FetchMany, Query},
    Sdk,
};
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    document::{Document, DocumentV0Getters},
    platform_value::{string_encoding::Encoding, Value},
    prelude::{DataContract, Identifier},
    system_data_contracts::{dashpay_contract, dpns_contract},
};
use drive::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use rs_dapi_client::DapiRequest;
use tokio::sync::Mutex;

use super::{
    as_json_string, state::KnownContractsMap, AppStateUpdate, Backend, BackendEvent, Task,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ContractTask {
    FetchDashpayContract,
    FetchDPNSContract,
    RemoveContract(String),
    FetchContract(String),
    OdyQuery,
}

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

                    let contract_name = get_dpns_name(sdk, &data_contract.id())
                        .await
                        .unwrap_or_else(|| data_contract.id().to_string(Encoding::Base58));

                    contracts_lock.insert(contract_name, data_contract);

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

                    let contract_name = get_dpns_name(sdk, &data_contract.id())
                        .await
                        .unwrap_or_else(|| data_contract.id().to_string(Encoding::Base58));

                    contracts_lock.insert(contract_name, data_contract);

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
        ContractTask::OdyQuery => {
            let known_contracts = known_contracts.lock().await;
            let dpns_contract = known_contracts
                .get(&dpns_contract::ID.to_string(Encoding::Base58))
                .unwrap();
            let domain_doc = dpns_contract.document_type_for_name("domain").unwrap();
            let query_asc = DriveDocumentQuery {
                contract: dpns_contract,
                document_type: domain_doc,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: Some(WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::LessThan,
                        value: Value::Identifier(
                            Identifier::from_string(
                                "AYN4srupPWDrp833iG5qtmaAsbapNvaV7svAdncLN5Rh",
                                Encoding::Base58,
                            )
                            .unwrap()
                            .to_buffer(),
                        ),
                    }),
                    equal_clauses: BTreeMap::new(),
                },
                offset: None,
                limit: Some(6),
                order_by: vec![(
                    "records.identity".to_string(),
                    OrderClause {
                        field: "records.identity".to_string(),
                        ascending: true,
                    },
                )]
                .into_iter()
                .collect(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };
            let query_desc = DriveDocumentQuery {
                contract: dpns_contract,
                document_type: domain_doc,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: Some(WhereClause {
                        field: "records.identity".to_string(),
                        operator: WhereOperator::LessThan,
                        value: Value::Identifier(
                            Identifier::from_string(
                                "AYN4srupPWDrp833iG5qtmaAsbapNvaV7svAdncLN5Rh",
                                Encoding::Base58,
                            )
                            .unwrap()
                            .to_buffer(),
                        ),
                    }),
                    equal_clauses: BTreeMap::new(),
                },
                offset: None,
                limit: Some(6),
                order_by: vec![(
                    "records.identity".to_string(),
                    OrderClause {
                        field: "records.identity".to_string(),
                        ascending: true,
                    },
                )]
                .into_iter()
                .collect(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
            };

            // let docquery_asc = DocumentQuery::new_with_drive_query(&query_asc);
            // let mut request_asc =
            //     GetDocumentsRequest::try_from(docquery_asc).expect("convert to proto");
            // if let Some(get_documents_request::Version::V0(ref mut v0_asc)) = request_asc.version {
            //     v0_asc.prove = false;
            // } else {
            //     panic!("version V0 not found");
            // };

            // let response_asc = request_asc
            //     .execute(&sdk, Default::default())
            //     .await
            //     .expect("fetch many documents");

            tracing::info!("Start query");

            let docquery_desc = DocumentQuery::new_with_drive_query(&query_desc);
            tracing::info!("1");

            let mut request_desc =
                GetDocumentsRequest::try_from(docquery_desc).expect("convert to proto");
            tracing::info!("2");

            if let Some(get_documents_request::Version::V0(ref mut v0_desc)) = request_desc.version
            {
                v0_desc.prove = false;
            } else {
                panic!("version V0 not found");
            };
            tracing::info!("3");

            let response_desc = request_desc
                .execute(&sdk, Default::default())
                .await
                .expect("fetch many documents");

            tracing::info!("4");

            // tracing::info!("ASC: {:?}", response_asc);
            tracing::info!("DESC: {:?}", response_desc);

            BackendEvent::None
        }
    }
}

pub async fn get_dpns_name(sdk: &Sdk, id: &Identifier) -> Option<String> {
    let dpns_contract =
        DataContract::fetch(&sdk, Into::<Identifier>::into(dpns_contract::ID_BYTES))
            .await
            .ok()??;

    let document_query = DocumentQuery {
        data_contract: Arc::new(dpns_contract),
        document_type_name: "domain".to_string(),
        where_clauses: vec![WhereClause {
            field: "label".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(id.to_buffer()),
        }],
        order_by_clauses: vec![],
        limit: 1,
        start: None,
    };

    let document = Document::fetch(sdk, document_query).await.ok()??;
    let properties = document.properties();
    let label = properties.get("label")?;

    Some(label.to_string())
}
