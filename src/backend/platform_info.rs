use crate::backend::{as_json_string, BackendEvent, Task};
use chrono::{prelude::*, LocalResult};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use dapi_grpc::platform::v0::{get_current_quorums_info_request, GetCurrentQuorumsInfoRequest, Proof, ResponseMetadata};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::sdk::prettify_proof;
use dash_sdk::{
    platform::{Fetch, FetchMany, LimitQuery},
    Sdk,
};
use dash_sdk::platform::FetchUnproved;
use dpp::block::epoch::Epoch;
use dpp::core_subsidy::NetworkCoreSubsidy;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    block::{
        epoch::EpochIndex,
        extended_epoch_info::{v0::ExtendedEpochInfoV0Getters, ExtendedEpochInfo},
    },
    version::ProtocolVersionVoteCount,
};
use drive_proof_verifier::types::{CurrentQuorumsInfo, NoParamQuery, ProtocolVersionUpgrades};

use drive::drive::credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT;
use drive::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use drive::drive::credit_pools::epochs::paths::EpochProposers;
use drive::drive::RootTree;
use drive::grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use drive_proof_verifier::types::TotalCreditsInPlatform;
use drive_proof_verifier::ContextProvider;
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
    FetchTotalCreditsOnPlatform,
    FetchCurrentVersionVotingState,
    FetchCurrentValidatorSetInfo,
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

    let readable_epoch_start_time_as_time_away =
        match Utc.timestamp_millis_opt(epoch_info.first_block_time() as i64) {
            LocalResult::None => String::new(),
            LocalResult::Single(block_time) => {
                // Get the current time for comparison
                let now = Utc::now();
                let duration = now.signed_duration_since(block_time);
                let human_readable =
                    HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Past);
                human_readable
            }
            LocalResult::Ambiguous(..) => String::new(),
        };

    let readable_epoch_start_time_as_time =
        match Utc.timestamp_millis_opt(epoch_info.first_block_time() as i64) {
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

fn format_current_quorums_info(current_quorums_info: &CurrentQuorumsInfo) -> String {
    format!(
        "CurrentQuorumsInfo {{\n
    quorum_hashes: [{}],\n
    current_quorum_hash: {},\n
    validator_sets: [{}],\n
    last_block_proposer: {},\n
    last_platform_block_height: {},\n
    last_core_block_height: {}\n
}}",
        current_quorums_info
            .quorum_hashes
            .iter()
            .map(|hash| hex::encode(hash))
            .collect::<Vec<_>>()
            .join(", "),
        hex::encode(current_quorums_info.current_quorum_hash),
        current_quorums_info.validator_sets.iter().join(", "),
        hex::encode(current_quorums_info.last_block_proposer),
        current_quorums_info.last_platform_block_height,
        current_quorums_info.last_core_block_height
    )
}

fn format_total_credits_on_platform(
    network: Network,
    request_activation_core_height: impl Fn() -> u32,
    total_credits_on_platform: TotalCreditsInPlatform,
    metadata: ResponseMetadata,
    proof: Proof,
) -> String {
    let grovedb_proof = &proof.grovedb_proof;
    let platform_version = PlatformVersion::latest();
    // we also need the path_query for the start_core_height of this unpaid epoch
    let unpaid_epoch_index = PathQuery {
        path: vec![vec![RootTree::Pools as u8]],
        query: SizedQuery {
            query: Query::new_single_key(KEY_UNPAID_EPOCH_INDEX.to_vec()),
            limit: Some(1),
            offset: None,
        },
    };

    let (_, mut proved_path_key_values) = GroveDb::verify_subset_query(
        grovedb_proof,
        &unpaid_epoch_index,
        &platform_version.drive.grove_version,
    )
    .expect("expected to verify subset query");

    let Some(proved_path_key_value) = proved_path_key_values.pop() else {
        return ("This proof would show that Platform has not yet been initialized as we can not find a start index").to_string();
    };

    if proved_path_key_value.0 != unpaid_epoch_index.path {
        return ("The result of this proof is not what we asked for (unpaid epoch path)")
            .to_string();
    }

    if proved_path_key_value.1 != KEY_UNPAID_EPOCH_INDEX.to_vec() {
        return ("The result of this proof is not what we asked for (unpaid epoch key)")
            .to_string();
    }

    let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
        return ("We are expecting an item for the epoch index").to_string();
    };

    let epoch_index = EpochIndex::from_be_bytes(
        bytes
            .as_slice()
            .try_into()
            .expect("epoch index invalid length"),
    );

    let epoch = Epoch::new(epoch_index).unwrap();

    let start_core_height_query = PathQuery {
        path: epoch.get_path_vec(),
        query: SizedQuery {
            query: Query::new_single_key(KEY_START_BLOCK_CORE_HEIGHT.to_vec()),
            limit: None,
            offset: None,
        },
    };

    let (_, mut proved_path_key_values) = GroveDb::verify_subset_query(
        grovedb_proof,
        &start_core_height_query,
        &platform_version.drive.grove_version,
    )
    .expect("expected to verify subset query");

    let Some(proved_path_key_value) = proved_path_key_values.pop() else {
        return ("We can not find the start core height of the unpaid epoch").to_string();
    };

    if proved_path_key_value.0 != start_core_height_query.path {
        return ("The result of this proof is not what we asked for (start core height path)")
            .to_string();
    }

    if proved_path_key_value.1 != KEY_START_BLOCK_CORE_HEIGHT.to_vec() {
        return ("The result of this proof is not what we asked for (start core height key)")
            .to_string();
    }

    let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
        return ("We are expecting an item for the start core height of the unpaid epoch")
            .to_string();
    };

    let start_core_height = u32::from_be_bytes(
        bytes
            .as_slice()
            .try_into()
            .expect("start core height invalid length"),
    );

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

    let dash_amount = total_credits_on_platform.0 as f64 * 10f64.powf(-11.0);

    let extra_start_info = if epoch.index == 0 {
        format!("activation height: {}\n", request_activation_core_height())
    } else {
        "".to_string()
    };

    format!(
        "current height: {}\ncurrent epoch start core height: {}\n{}current core height: {}\nsubsidy interval: {}\ncurrent block time: {} ({})\ntotal_credits_on_platform {} ({:.4} Dash)\n\nproof: {}",
        metadata.height,
        start_core_height,
        extra_start_info,
        metadata.core_chain_locked_height,
        network.core_subsidy_halving_interval(),
        metadata.time_ms,
        readable_block_time,
        total_credits_on_platform.0,
        dash_amount,
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
                        epoch_info,
                        metadata,
                        proof,
                        sdk.network,
                        true,
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
                        epoch_info,
                        metadata,
                        proof,
                        sdk.network,
                        false,
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
                    let votes: ProtocolVersionUpgrades = votes;

                    let votes_info = votes
                        .into_iter()
                        .map(|(key, value): (u32, Option<u64>)| {
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
        PlatformInfoTask::FetchTotalCreditsOnPlatform => {
            match TotalCreditsInPlatform::fetch_current_with_metadata_and_proof(sdk).await {
                Ok((epoch_info, metadata, proof)) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok(format_total_credits_on_platform(
                        sdk.network,
                        || {
                            sdk.context_provider()
                                .expect("expected context provider")
                                .get_platform_activation_height()
                                .expect("expected platform activation height")
                        },
                        epoch_info,
                        metadata,
                        proof,
                    )
                    .into()),
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
        PlatformInfoTask::FetchCurrentValidatorSetInfo => {
            match CurrentQuorumsInfo::fetch_unproved(sdk, NoParamQuery{}).await {
                Ok(Some(current_quorums_info)) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok(format_current_quorums_info(&current_quorums_info).into()),
                },
                Ok(None) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Ok("No current quorums".into()),
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
    }
}
