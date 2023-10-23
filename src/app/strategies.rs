use std::collections::BTreeMap;

use bincode::{Encode, Decode};
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::{version::PlatformVersion, serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure, ProtocolError, prelude::Identity, state_transition::StateTransition};
use dpp::ProtocolError::PlatformDeserializationError;
use serde::Serialize;
use simple_signer::signer::SimpleSigner;
use strategy_tests::operations::Operation;
use strategy_tests::{Strategy, frequency::Frequency};

#[derive(Debug, Clone)]
pub struct StrategyDetails {
    pub(crate) strategy: Strategy,
    pub(crate) description: String,
}

#[derive(Clone, Debug, Encode, Decode)]
struct StrategyInSerializationFormat {
    pub contracts_with_updates: Vec<(Vec<u8>, Option<BTreeMap<u64, Vec<u8>>>)>,
    pub operations: Vec<Vec<u8>>,
    pub start_identities: Vec<(Identity, StateTransition)>,
    pub identities_inserts: Frequency,
    pub signer: Option<SimpleSigner>,
}

#[derive(Clone, Debug, Encode, Decode)]
struct StrategyDetailsInSerializationFormat {
    strategy: StrategyInSerializationFormat,
    description: String
}


impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for StrategyDetails {
    fn versioned_deserialize(
        data: &[u8],
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let strategy: StrategyDetailsInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize Strategy: {}", e))
                })?
                .0;

        let StrategyDetailsInSerializationFormat {
            strategy: StrategyInSerializationFormat {
                contracts_with_updates,
                operations,
                start_identities,
                identities_inserts,
                signer,
            },
            description
        } = strategy;

        let contracts_with_updates = contracts_with_updates
            .into_iter()
            .map(|(serialized_contract, maybe_updates)| {
                let contract = CreatedDataContract::versioned_deserialize(
                    serialized_contract.as_slice(),
                    validate,
                    platform_version,
                )?;
                let maybe_updates = maybe_updates
                    .map(|updates| {
                        updates
                            .into_iter()
                            .map(|(key, serialized_contract_update)| {
                                let update = CreatedDataContract::versioned_deserialize(
                                    serialized_contract_update.as_slice(),
                                    validate,
                                    platform_version,
                                )?;
                                Ok((key, update))
                            })
                            .collect::<Result<BTreeMap<u64, CreatedDataContract>, ProtocolError>>()
                    })
                    .transpose()?;
                Ok((contract, maybe_updates))
            })
            .collect::<Result<
                Vec<(
                    CreatedDataContract,
                    Option<BTreeMap<u64, CreatedDataContract>>,
                )>,
                ProtocolError,
            >>()?;

        let operations = operations
            .into_iter()
            .map(|operation| {
                Operation::versioned_deserialize(operation.as_slice(), validate, platform_version)
            })
            .collect::<Result<Vec<Operation>, ProtocolError>>()?;

        Ok(StrategyDetails {
            strategy: Strategy {
                contracts_with_updates,
                operations,
                start_identities,
                identities_inserts,
                signer,
            },
            description: description
        })
    }
}
