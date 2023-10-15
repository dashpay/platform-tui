use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use bincode::{Decode, Encode};
use dpp::prelude::{DataContract, Identity};
use dpp::ProtocolError;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use dpp::serialization::{PlatformDeserializableWithPotentialValidationFromVersionedStructure, PlatformSerializableWithPlatformVersion};
use dpp::version::PlatformVersion;
use strategy_tests::Strategy;
use crate::app::wallet::Wallet;

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
    fn load() -> AppState{
        let path = Path::new("explorer.config");

        let read_result = fs::read(path);
        let config = match read_result {
            Ok(data) => bincode::decode_from_slice(&data).expect("config file is corrupted"),
            Err(_) => HashMap::new(),
        };

        let path = Path::new("explorer.contracts");

        let read_result = fs::read(path);
        let contract_paths: BTreeMap<String, String> = match read_result {
            Ok(data) => bincode::decode_from_slice(&data).expect("contracts file is corrupted"),
            Err(_) => BTreeMap::new(),
        };

        let available_contracts = contract_paths
            .iter()
            .filter_map(|(alias, path)| {
                open_contract(&platform.drive, path)
                    .map_or(None, |contract| Some((alias.clone(), contract)))
            })
            .collect();

        let path = Path::new("explorer.strategies");

        let read_result = fs::read(path);
        let available_strategies: BTreeMap<String, Strategy> = match read_result {
            Ok(data) => bincode::decode_from_slice(&data).expect("strategies file is corrupted"),
            Err(_) => BTreeMap::new(),
        };

        AppState {
            screen: MainScreen,
            last_block: None,
            current_epoch: None,
            masternodes: IndexMap::default(),
            current_execution_strategy: None,
            config,
            contract_paths,
            available_contracts,
            available_strategies,
        }
    }

    fn save_config(&self) {
        let config = bincode::serialize(&self.config).expect("unable to serialize config");
        let path = Path::new("explorer.config");

        fs::write(path, config).unwrap();
    }

    fn save_available_contracts(&self) {
        let contracts =
            bincode::serialize(&self.contract_paths).expect("unable to serialize contract paths");
        let path = Path::new("explorer.contracts");

        fs::write(path, contracts).unwrap();
    }

    fn save_available_strategies(&self) {
        let strategies =
            bincode::serialize(&self.available_strategies).expect("unable to serialize strategies");
        let path = Path::new("explorer.strategies");

        fs::write(path, strategies).unwrap();
    }

    fn load_last_contract(&self, drive: &Drive) -> Option<Contract> {
        let last_contract_path = self.config.get(LAST_CONTRACT_PATH)?;
        let db_transaction = drive.grove.start_transaction();

        let mut rng = rand::rngs::StdRng::from_entropy();
        let contract_id = rng.gen::<[u8; 32]>();
        let contract = common::setup_contract(
            &drive,
            last_contract_path,
            Some(contract_id),
            Some(&db_transaction),
        );
        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("expected to commit transaction");
        Some(contract)
    }

    fn load_contract(&mut self, drive: &Drive, contract_path: &str) -> Result<Contract, Error> {
        let db_transaction = drive.grove.start_transaction();

        let mut rng = rand::rngs::StdRng::from_entropy();
        let contract_id = rng.gen::<[u8; 32]>();
        let contract = common::setup_contract(
            &drive,
            contract_path,
            Some(contract_id),
            Some(&db_transaction),
        );
        drive.commit_transaction(db_transaction)?;
        self.config
            .insert(LAST_CONTRACT_PATH.to_string(), contract_path.to_string());
        self.save_config();
        Ok(contract)
    }
}