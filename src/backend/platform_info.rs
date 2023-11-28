use dash_platform_sdk::{platform::Fetch, Sdk};
use dpp::block::extended_epoch_info::ExtendedEpochInfo;

use crate::backend::{as_toml, BackendEvent, Task};

#[derive(Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
}
pub(super) async fn run_platform_task<'s>(sdk: &Sdk, task: PlatformInfoTask) -> BackendEvent<'s> {
    match task {
        PlatformInfoTask::FetchCurrentEpochInfo => match ExtendedEpochInfo::fetch(sdk, 5).await {
            Ok(Some(epoch_info)) => {
                let epoch_info = as_toml(&epoch_info);

                BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok(epoch_info.into()),
                }
            }
            Ok(None) => BackendEvent::TaskCompleted {
                task: Task::PlatformInfo(task),
                execution_result: Ok("No epoch".into()),
            },
            Err(e) => BackendEvent::TaskCompleted {
                task: Task::PlatformInfo(task),
                execution_result: Err(e.to_string()),
            },
        },
    }
}
