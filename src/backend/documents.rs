mod broadcast_random_documents;

use dpp::{data_contract::accessors::v0::DataContractV0Getters, document::Document};
use rs_sdk::{
    platform::{DocumentQuery, FetchMany},
    Sdk,
};

use super::{AppStateUpdate, CompletedTaskPayload};
pub(crate) use crate::backend::{AppState, BackendEvent, Task};

#[derive(Debug, Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
    BroadcastRandomDocuments {
        data_contract_name: String,
        document_type_name: String,
        count: u16,
    },
}

impl AppState {
    pub(super) async fn run_document_task<'s>(
        &'s self,
        sdk: &Sdk,
        task: DocumentTask,
    ) -> BackendEvent<'s> {
        match &task {
            DocumentTask::QueryDocuments(document_query) => {
                let execution_result = Document::fetch_many(&sdk, document_query.clone())
                    .await
                    .map(CompletedTaskPayload::Documents)
                    .map_err(|e| e.to_string());
                BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result,
                }
            }
            DocumentTask::BroadcastRandomDocuments {
                data_contract_name,
                document_type_name,
                count,
            } => {
                let broadcast_stats = {
                    let data_contracts_lock = self.known_contracts.lock().await;
                    let Some(data_contract) = data_contracts_lock.get(data_contract_name) else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("Data contract not found".to_owned()),
                        };
                    };
                    let loaded_identity_lock = self.loaded_identity.lock().await;
                    let Some(identity) = loaded_identity_lock.as_ref() else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("No loaded identity".to_owned()),
                        };
                    };
                    let identity_private_keys_lock = self.identity_private_keys.lock().await;
                    let Ok(document_type) =
                        data_contract.document_type_cloned_for_name(&document_type_name)
                    else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("Document type not found".to_owned()),
                        };
                    };

                    broadcast_random_documents::broadcast_random_documents(
                        sdk,
                        identity,
                        &identity_private_keys_lock,
                        data_contract,
                        &document_type,
                        *count,
                    )
                    .await
                };

                match broadcast_stats {
                    Ok(stats) => match self.refresh_identity(sdk).await {
                        Ok(updated_identity) => BackendEvent::TaskCompletedStateChange {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                stats.info_display(),
                            )),
                            app_state_update: AppStateUpdate::LoadedIdentity(updated_identity),
                        },
                        Err(_) => BackendEvent::TaskCompletedStateChange {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                stats.info_display(),
                            )),
                            app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                        },
                    },
                    Err(e) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!(
                                "Unable to start broadcasting: {}",
                                e.to_string()
                            )),
                        }
                    }
                }
            }
        }
    }
}
