use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{prelude::*, LocalResult};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::sdk::prettify_proof;
use dash_sdk::{
    platform::{Fetch, FetchMany, LimitQuery},
    Sdk,
};
use dash_sdk::{RequestSettings, SdkBuilder};
use dpp::node::status::v0::EvonodeStatusV0Getters;
use dpp::node::status::EvonodeStatus;
use dpp::version::PlatformVersion;
use dpp::{
    block::{
        epoch::EpochIndex,
        extended_epoch_info::{v0::ExtendedEpochInfoV0Getters, ExtendedEpochInfo},
    },
    version::ProtocolVersionVoteCount,
};
use futures::future::join_all;
use rs_dapi_client::AddressList;
use tokio::task;

use crate::backend::{as_json_string, BackendEvent, Task};
use crate::config::Config;

use super::CompletedTaskPayload;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
    FetchCurrentVersionVotingState,
    FetchSpecificEpochInfo(u16),
    FetchManyEpochInfo(u16, u32), // second is count
    FetchNodeStatuses,
}

fn format_extended_epoch_info(
    epoch_info: ExtendedEpochInfo,
    metadata: ResponseMetadata,
    proof: Proof,
    is_current: bool,
) -> String {
    let readable_block_time = match Utc.timestamp_millis_opt(metadata.time_ms as i64) {
        LocalResult::None => String::new(),
        LocalResult::Single(block_time) => {
            // Get the current time for comparison
            let now = Utc::now();
            let duration = now.signed_duration_since(block_time);
            let human_readable = HumanTime::from(duration).to_text_en(Accuracy::Rough, Tense::Past);
            human_readable
        }
        LocalResult::Ambiguous(..) => String::new(),
    };

    let readable_epoch_start_time = match Utc
        .timestamp_millis_opt(epoch_info.first_block_time() as i64)
    {
        LocalResult::None => String::new(),
        LocalResult::Single(block_time) => {
            // Get the current time for comparison
            let now = Utc::now();
            let duration = now.signed_duration_since(block_time);
            let human_readable = HumanTime::from(duration).to_text_en(Accuracy::Rough, Tense::Past);
            human_readable
        }
        LocalResult::Ambiguous(..) => String::new(),
    };

    let in_string = if is_current {
        "in ".to_string()
    } else {
        String::default()
    };

    format!(
        "current height: {}\ncurrent core height: {}\ncurrent block time: {} ({})\n{}epoch: {}\n \
         * start height: {}\n * start core height: {}\n * start time: {} ({})\n * fee multiplier: \
         {}\n\nproof: {}",
        metadata.height,
        metadata.core_chain_locked_height,
        metadata.time_ms,
        readable_block_time,
        in_string,
        epoch_info.index(),
        epoch_info.first_block_height(),
        epoch_info.first_core_block_height(),
        epoch_info.first_block_time(),
        readable_epoch_start_time,
        epoch_info.fee_multiplier_permille(),
        prettify_proof(&proof)
    )
}

pub(super) async fn run_platform_task<'s>(sdk: &Sdk, task: PlatformInfoTask) -> BackendEvent<'s> {
    match task {
        PlatformInfoTask::FetchCurrentEpochInfo => {
            match ExtendedEpochInfo::fetch_current_with_metadata_and_proof(sdk).await {
                Ok((epoch_info, metadata, proof)) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok(format_extended_epoch_info(
                        epoch_info, metadata, proof, true,
                    )
                    .into()),
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
        PlatformInfoTask::FetchSpecificEpochInfo(epoch_num) => {
            match ExtendedEpochInfo::fetch_with_metadata_and_proof(sdk, epoch_num, None).await {
                Ok((Some(epoch_info), metadata, proof)) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok(format_extended_epoch_info(
                        epoch_info, metadata, proof, false,
                    )
                    .into()),
                },
                Ok((None, _, proof)) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: {
                        Ok(format!("No epoch, \n proof {}", prettify_proof(&proof)).into())
                    },
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
                start_info: None,
                limit: Some(limit),
            };

            match ExtendedEpochInfo::fetch_many(&sdk, query).await {
                Ok(epoch_infos) => {
                    let epoch_info = as_json_string(&epoch_infos);

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
            match ProtocolVersionVoteCount::fetch_many(&sdk, ()).await {
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
        PlatformInfoTask::FetchNodeStatuses => {
            let address_list = match sdk.address_list() {
                Ok(list) => list,
                Err(e) => {
                    return BackendEvent::TaskCompleted {
                        task: Task::PlatformInfo(task),
                        execution_result: Err(format!(
                            "Failed to fetch DapiClient address list: {e}"
                        )),
                    }
                }
            };

            tracing::info!("Address list length: {}", address_list.len());
            tracing::info!("Address list: {:?}", address_list);

            let config = Config::load();
            let request_settings = RequestSettings {
                connect_timeout: Some(Duration::from_secs(10)),
                timeout: Some(Duration::from_secs(10)),
                retries: None,
                ban_failed_address: Some(false),
            };

            // Create a vector to hold the handles for all the spawned tasks
            let mut handles = Vec::new();

            for address in address_list.addresses() {
                let address = address.uri().clone();
                let config = config.clone();
                let request_settings = request_settings.clone();
                let mut single_address_list = AddressList::new();
                single_address_list.add_uri(address);

                // Spawn a task for each iteration
                let handle = task::spawn(async move {
                    let sdk = SdkBuilder::new(single_address_list.clone())
                        .with_version(PlatformVersion::get(1).unwrap())
                        .with_core(
                            &config.core_host,
                            config.core_rpc_port,
                            &config.core_rpc_user,
                            &config.core_rpc_password,
                        )
                        .with_settings(request_settings)
                        .build()
                        .expect("expected to build sdk");

                    match EvonodeStatus::fetch(&sdk, ()).await {
                        Ok(result) => {
                            if let Some(status) = result {
                                Some((status.pro_tx_hash(), status))
                            } else {
                                tracing::info!("None result from fetching EvonodeStatus");
                                None
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Error fetching node status: {e}");
                            None
                        }
                    }
                });

                handles.push(handle); // Push each spawned task's handle into the vector
            }

            // Await all tasks to finish
            let results = join_all(handles).await;

            let mut node_statuses = BTreeMap::new();
            for result in results {
                if let Ok(Some((pro_tx_hash, node_status))) = result {
                    node_statuses.insert(pro_tx_hash, node_status);
                }
            }

            BackendEvent::TaskCompleted {
                task: Task::PlatformInfo(task),
                execution_result: Ok(CompletedTaskPayload::EvonodeStatuses(node_statuses)),
            }
        }
    }
}
