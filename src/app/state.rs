use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use bincode::{Decode, Encode};
use dpp::prelude::{DataContract, Identity};
use dpp::ProtocolError;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use dpp::serialization::{PlatformDeserializableWithPotentialValidationFromVersionedStructure, PlatformSerializableWithPlatformVersion};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use strategy_tests::Strategy;
use crate::app::wallet::Wallet;

const CURRENT_PROTOCOL_VERSION: ProtocolVersion = 1;

#[derive(Debug, Default)]
pub struct AppState {
    pub loaded_identity : Option<Identity>,
    pub loaded_wallet: Option<Wallet>,
    pub known_identities: BTreeMap<String, Identity>,
    pub known_contracts: BTreeMap<String, DataContract>,
    pub available_strategies: BTreeMap<String, Strategy>,
}


#[derive(Clone, Debug, Encode, Decode)]
struct AppStateInSerializationFormat {
    pub loaded_identity : Option<Identity>,
    pub loaded_wallet: Option<Wallet>,
    pub known_identities: BTreeMap<String, Identity>,
    pub known_contracts: BTreeMap<String, Vec<u8>>,
    pub available_strategies: BTreeMap<String, Vec<u8>>,
}

impl PlatformSerializableWithPlatformVersion for AppState {
    type Error = ProtocolError;

    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.clone()
            .serialize_consume_to_bytes_with_platform_version(platform_version)
    }

    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        let AppState {
            loaded_identity, loaded_wallet, known_identities, known_contracts, available_strategies
        } = self;

        let known_contracts_in_serialization_format = known_contracts
            .into_iter()
            .map(|(key, contract)| {
                let serialized_contract = contract.serialize_consume_to_bytes_with_platform_version(platform_version)?;
                Ok((key, serialized_contract))
            })
            .collect::<Result<BTreeMap<String, Vec<u8>>, ProtocolError>>()?;

        let available_strategies_in_serialization_format = available_strategies
            .into_iter()
            .map(|(key, strategy)| {
                let serialized_strategy = strategy.serialize_consume_to_bytes_with_platform_version(platform_version)?;
                Ok((key, serialized_strategy))
            })
            .collect::<Result<BTreeMap<String, Vec<u8>>, ProtocolError>>()?;

        let app_state_in_serialization_format = AppStateInSerializationFormat {
            loaded_identity,
            loaded_wallet,
            known_identities,
            known_contracts: known_contracts_in_serialization_format,
            available_strategies: available_strategies_in_serialization_format,
        };

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        bincode::encode_to_vec(app_state_in_serialization_format, config)
            .map_err(|e| PlatformSerializationError(format!("unable to serialize App State: {}", e)))
    }
}

impl PlatformDeserializableWithPotentialValidationFromVersionedStructure for AppState {
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
        let app_state: AppStateInSerializationFormat =
            bincode::borrow_decode_from_slice(data, config)
                .map_err(|e| {
                    PlatformDeserializationError(format!("unable to deserialize App State: {}", e))
                })?
                .0;

        let AppStateInSerializationFormat {
            loaded_identity, loaded_wallet, known_identities, known_contracts, available_strategies
        } = app_state;

        let known_contracts = known_contracts
            .into_iter()
            .map(|(key, contract)| {
                let contract = DataContract::versioned_deserialize(contract.as_slice(), validate, platform_version)?;
                Ok((key, contract))
            })
            .collect::<Result<BTreeMap<String, DataContract>, ProtocolError>>()?;

        let available_strategies = available_strategies
            .into_iter()
            .map(|(key, strategy)| {
                let strategy = Strategy::versioned_deserialize(strategy.as_slice(), validate, platform_version)?;
                Ok((key, strategy))
            })
            .collect::<Result<BTreeMap<String, Strategy>, ProtocolError>>()?;

        Ok(AppState {
            loaded_identity,
            loaded_wallet,
            known_identities,
            known_contracts,
            available_strategies,
        })
    }
}

impl AppState {
    pub fn load() -> AppState{
        let path = Path::new("explorer.state");

        let Ok(read_result) = fs::read(path) else {
            return AppState::default()
        };

        let Ok(app_state) = AppState::versioned_deserialize(read_result.as_slice(), false, PlatformVersion::get(CURRENT_PROTOCOL_VERSION).unwrap()) else {
            return AppState::default()
        };

        app_state
    }

    pub fn save(&self) {
        let platform_version = PlatformVersion::get(CURRENT_PROTOCOL_VERSION).unwrap();
        let path = Path::new("explorer.state");

        let serialized_state = self.serialize_to_bytes_with_platform_version(platform_version).expect("expected to save state");
        fs::write(path, serialized_state).unwrap();
    }
}