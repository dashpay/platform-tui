use rs_sdk::{
    platform::{types::epoch::ExtendedEpochInfoEx, Fetch, FetchMany, LimitQuery},
    Sdk,
};
use dpp::{
    block::{epoch::EpochIndex, extended_epoch_info::ExtendedEpochInfo},
    version::ProtocolVersionVoteCount,
};

use crate::backend::{as_toml, BackendEvent, Task};

#[derive(Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
    FetchCurrentVersionVotingState,
    FetchSpecificEpochInfo(u16),
    FetchManyEpochInfo(u16, u32), // second is count
}
pub(super) async fn run_platform_task<'s>(sdk: &Sdk, task: PlatformInfoTask) -> BackendEvent<'s> {
    match task {
        PlatformInfoTask::FetchCurrentEpochInfo => {
            match ExtendedEpochInfo::fetch_current(sdk).await {
                Ok(epoch_info) => {
                    let epoch_info = as_toml(&epoch_info);

                    BackendEvent::TaskCompleted {
                        task: Task::PlatformInfo(task),
                        execution_result: Ok(epoch_info.into()),
                    }
                }
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
        PlatformInfoTask::FetchSpecificEpochInfo(epoch_num) => {
            match ExtendedEpochInfo::fetch(sdk, epoch_num).await {
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
            }
        }
        PlatformInfoTask::FetchManyEpochInfo(epoch_num, limit) => {
            let query: LimitQuery<EpochIndex> = LimitQuery {
                query: epoch_num,
                limit: Some(limit),
            };

            match ExtendedEpochInfo::fetch_many(sdk, query).await {
                Ok(epoch_infos) => {
                    let epoch_info = as_toml(&epoch_infos);

                    BackendEvent::TaskCompleted {
                        task: Task::PlatformInfo(task),
                        execution_result: Ok(epoch_info.into()),
                    }
                }
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
        PlatformInfoTask::FetchCurrentVersionVotingState => {
            match ProtocolVersionVoteCount::fetch_many(sdk, ()).await {
                Ok(votes) => {
                    let votes_info = votes
                        .into_iter()
                        .map(|(key, value)| {
                            format!(
                                "Version {} -> {}",
                                key,
                                value
                                    .map(|v| format!("{} votes", v))
                                    .unwrap_or("No votes".to_string())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    BackendEvent::TaskCompleted {
                        task: Task::PlatformInfo(task),
                        execution_result: Ok(votes_info.into()),
                    }
                }
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
    }
}
