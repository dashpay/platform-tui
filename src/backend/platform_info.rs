use crate::backend::{as_json_string, BackendEvent, Task};
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
use dpp::block::epoch::Epoch;
use dpp::core_subsidy::NetworkCoreSubsidy;
use dpp::dashcore::Network;
use drive_proof_verifier::ContextProvider;
use dpp::version::PlatformVersion;
use drive::drive::credit_pools::epochs::epoch_key_constants::KEY_START_BLOCK_CORE_HEIGHT;
use drive::drive::RootTree;
use drive::drive::credit_pools::epochs::epochs_root_tree_key_constants::KEY_UNPAID_EPOCH_INDEX;
use drive::drive::credit_pools::epochs::paths::EpochProposers;
use drive::grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use drive_proof_verifier::types::TotalCreditsOnPlatform;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlatformInfoTask {
    FetchCurrentEpochInfo,
    FetchTotalCreditsOnPlatform,
    FetchCurrentVersionVotingState,
    FetchSpecificEpochInfo(u16),
    FetchManyEpochInfo(u16, u32), // second is count
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

fn format_total_credits_on_platform(
    network: Network,
    request_activation_core_height: impl Fn() -> u32,
    total_credits_on_platform: TotalCreditsOnPlatform,
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
    ).expect("expected to verify subset query");

    let Some(proved_path_key_value) = proved_path_key_values.pop() else {
        return panic!("This proof would show that Platform has not yet been initialized as we can not find a start index");
    };

    if proved_path_key_value.0 != unpaid_epoch_index.path {
        return panic!("The result of this proof is not what we asked for (unpaid epoch path)");
    }

    if proved_path_key_value.1 != KEY_UNPAID_EPOCH_INDEX.to_vec() {
        return panic!("The result of this proof is not what we asked for (unpaid epoch key)");
    }

    let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
        return panic!("We are expecting an item for the epoch index");
    };

    let epoch_index = EpochIndex::from_be_bytes(bytes.as_slice().try_into().expect(
            "epoch index invalid length"
        ));

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
    ).expect("expected to verify subset query");

    let Some(proved_path_key_value) = proved_path_key_values.pop() else {
        return panic!("We can not find the start core height of the unpaid epoch");
    };

    if proved_path_key_value.0 != start_core_height_query.path {
        return panic!("The result of this proof is not what we asked for (start core height path)");
    }

    if proved_path_key_value.1 != KEY_START_BLOCK_CORE_HEIGHT.to_vec() {
        return panic!("The result of this proof is not what we asked for (start core height key)");
    }

    let Some(Element::Item(bytes, _)) = proved_path_key_value.2 else {
        return panic!("We are expecting an item for the start core height of the unpaid epoch");
    };

    let start_core_height = u32::from_be_bytes(bytes.as_slice().try_into().expect(
            "start core height invalid length"));

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
        PlatformInfoTask::FetchTotalCreditsOnPlatform => {
            match TotalCreditsOnPlatform::fetch_current_with_metadata_and_proof(sdk).await {
                Ok((epoch_info, metadata, proof)) => {
                    BackendEvent::TaskCompleted {
                        task: Task::PlatformInfo(task),
                        execution_result: Ok(format_total_credits_on_platform(
                            sdk.network,
                            || {
                                sdk.context_provider().expect("expected context provider").get_platform_activation_height().expect("expected platform activation height")
                            },
                            epoch_info, metadata, proof,
                        )
                            .into()),
                    }
                },
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::PlatformInfo(task),
                    execution_result: Err(e.to_string()),
                },
            }
        }
    }
}
