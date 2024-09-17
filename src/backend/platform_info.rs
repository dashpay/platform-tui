use chrono::{prelude::*, LocalResult};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::sdk::prettify_proof;
use dash_sdk::{
    platform::{Fetch, FetchMany, LimitQuery},
    Sdk,
};
use dpp::{
    block::{
        epoch::EpochIndex,
        extended_epoch_info::{v0::ExtendedEpochInfoV0Getters, ExtendedEpochInfo},
    },
    version::ProtocolVersionVoteCount,
};
use dpp::dashcore::Network;
use drive::config::DriveConfig;
use crate::backend::{as_json_string, BackendEvent, Task};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
    FetchCurrentVersionVotingState,
    FetchSpecificEpochInfo(u16),
    FetchManyEpochInfo(u16, u32), // second is count
}

fn format_extended_epoch_info(
    epoch_info: ExtendedEpochInfo,
    metadata: ResponseMetadata,
    proof: Proof,
    network: Network,
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

    let readable_epoch_start_time_as_time_away = match Utc
        .timestamp_millis_opt(epoch_info.first_block_time() as i64)
    {
        LocalResult::None => String::new(),
        LocalResult::Single(block_time) => {
            // Get the current time for comparison
            let now = Utc::now();
            let duration = now.signed_duration_since(block_time);
            let human_readable = HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Past);
            human_readable
        }
        LocalResult::Ambiguous(..) => String::new(),
    };

    let readable_epoch_start_time_as_time = match Utc.timestamp_millis_opt(epoch_info.first_block_time() as i64) {
        LocalResult::None => String::new(),
        LocalResult::Single(block_time) => {
            // Convert block_time to local time
            let local_time = block_time.with_timezone(&Local);

            // Format the local time in ISO 8601 format
            local_time.to_rfc2822()
        }
        LocalResult::Ambiguous(..) => String::new(),
    };

    let in_string = if is_current {
        "in ".to_string()
    } else {
        String::default()
    };

    let epoch_estimated_time = match network {
        Network::Dash => 788_400_000,
        Network::Testnet => 3_600_000,
        Network::Devnet => 3_600_000,
        Network::Regtest => 1_200_000,
        _ => 3_600_000,
    };

    let readable_epoch_end_time = match Utc
        .timestamp_millis_opt(epoch_info.first_block_time() as i64 + epoch_estimated_time as i64)
    {
        LocalResult::None => String::new(),
        LocalResult::Single(block_time) => {
            // Get the current time for comparison
            let now = Utc::now();
            let duration = block_time.signed_duration_since(now);

            let human_readable = if duration.num_milliseconds() >= 0 {
                // Time is in the future
                HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Future)
            } else {
                // Time is in the past
                HumanTime::from(-duration).to_text_en(Accuracy::Precise, Tense::Past)
            };

            human_readable
        }
        LocalResult::Ambiguous(..) => String::new(),
    };

    format!(
        "protocol version: {}\n current height: {}\ncurrent core height: {}\ncurrent block time: {} ({})\n{}epoch: {}\n \
         * start height: {}\n * start core height: {}\n * start time: {} ({} - {})\n * estimated end time: {} ({})\n * fee multiplier: \
         {}\n\nproof: {}",
        epoch_info.protocol_version(),
        metadata.height,
        metadata.core_chain_locked_height,
        metadata.time_ms,
        readable_block_time,
        in_string,
        epoch_info.index(),
        epoch_info.first_block_height(),
        epoch_info.first_core_block_height(),
        epoch_info.first_block_time(),
        readable_epoch_start_time_as_time,
        readable_epoch_start_time_as_time_away,
        epoch_info.first_block_time() + epoch_estimated_time,
        readable_epoch_end_time,
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
                        epoch_info, metadata, proof, sdk.network, true,
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
                        epoch_info, metadata, proof, sdk.network, false,
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
    }
}
