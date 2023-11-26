use dash_platform_sdk::platform::{DocumentQuery, Fetch, FetchMany};
use dash_platform_sdk::Sdk;
use dpp::document::Document;
use crate::backend::{as_toml, BackendEvent, stringify_result, Task};

#[derive(Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
}


pub(super) async fn run_document_task<'s>(
    sdk: &mut Sdk,
    task: DocumentTask,
) -> BackendEvent<'s> {
    match &task {
        DocumentTask::QueryDocuments(document_query) => {
            match Document::fetch_many(sdk, document_query.clone())
                .await
            {
                Ok(documents) => {
                    BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Ok(as_toml(&documents)),
                    }
                }
                result => BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result: stringify_result(result),
                },
            }
        }
    }
}
