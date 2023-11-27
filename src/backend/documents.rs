use dash_platform_sdk::{
    platform::{DocumentQuery, FetchMany},
    Sdk,
};
use dpp::document::Document;

use crate::backend::{stringify_result, BackendEvent, Task};

#[derive(Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
}

pub(super) async fn run_document_task<'s>(sdk: &mut Sdk, task: DocumentTask) -> BackendEvent<'s> {
    match &task {
        DocumentTask::QueryDocuments(document_query) => {
            let result = Document::fetch_many(sdk, document_query.clone()).await;
            BackendEvent::TaskCompleted {
                task: Task::Document(task),
                execution_result: stringify_result(result),
            }
        }
    }
}
