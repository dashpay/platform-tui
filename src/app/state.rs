use std::collections::BTreeMap;
use walkdir::{WalkDir, DirEntry};
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use bincode::{Decode, Encode};
use dpp::prelude::Identity;
use dpp::data_contract::created_data_contract::CreatedDataContract;
use dpp::ProtocolError;
use dpp::ProtocolError::{PlatformDeserializationError, PlatformSerializationError};
use dpp::serialization::{PlatformDeserializableWithPotentialValidationFromVersionedStructure, PlatformSerializableWithPlatformVersion};
use dpp::tests::json_document::json_document_to_created_contract;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::PlatformVersion;
use strategy_tests::Strategy;
use strategy_tests::frequency::Frequency;
use tokio::task;
use crate::app::wallet::Wallet;
use crate::app::strategies::StrategyDetails;

const CURRENT_PROTOCOL_VERSION: ProtocolVersion = 1;

#[derive(Debug, Clone)]
pub struct AppState {
    pub loaded_identity : Option<Identity>,
    pub loaded_wallet: Option<Arc<Wallet>>,
    pub known_identities: BTreeMap<String, Identity>,
    pub known_contracts: BTreeMap<String, CreatedDataContract>,
    pub available_strategies: BTreeMap<String, StrategyDetails>,
    pub current_strategy: Option<String>,
    pub selected_strategy: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut known_contracts = BTreeMap::new();
        let mut available_strategies = BTreeMap::new();
        
        let platform_version = PlatformVersion::latest();

        fn is_json(entry: &DirEntry) -> bool {
            entry.path().extension().and_then(|s| s.to_str()) == Some("json")
        }

        for entry in WalkDir::new("supporting_files/contract")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(is_json) 
        {
            let path = entry.path();
            let contract_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            
            let contract = json_document_to_created_contract(
                &path, 
                true,
                platform_version,
            ).expect("expected to get contract from a json document");

            known_contracts.insert(contract_name, contract);
        }

        let default_strategy_1 = StrategyDetails {
            strategy: Strategy {
                    contracts_with_updates: vec![(known_contracts.get(&String::from("dashpay-contract-all-mutable")).unwrap().clone(), None)],
                    operations: vec![],
                    start_identities: vec![],
                    identities_inserts: Frequency {
                        times_per_block_range: Default::default(),
                        chance_per_block: None,
                    },
                    signer: None,
                },
            description: "default everything with dashpay contract 1".to_string()
        };
        let default_strategy_2 = StrategyDetails {
            strategy: Strategy {
                    contracts_with_updates: vec![(known_contracts.get(&String::from("dashpay-contract-all-mutable-update-1")).unwrap().clone(), None)],
                    operations: vec![],
                    start_identities: vec![],
                    identities_inserts: Frequency {
                        times_per_block_range: Default::default(),
                        chance_per_block: None,
                    },
                    signer: None,
                },
            description: "default everything with dashpay contract 2".to_string()
        };
        let default_strategy_3 = StrategyDetails {
            strategy: Strategy {
                    contracts_with_updates: vec![(known_contracts.get(&String::from("dashpay-contract-all-mutable-update-2")).unwrap().clone(), None)],
                    operations: vec![],
                    start_identities: vec![],
                    identities_inserts: Frequency {
                        times_per_block_range: Default::default(),
                        chance_per_block: None,
                    },
                    signer: None,
                },
            description: "default everything with dashpay contract 3".to_string()
        };
        
        available_strategies.insert(String::from("default_strategy_1"), default_strategy_1);
        available_strategies.insert(String::from("default_strategy_2"), default_strategy_2);
        available_strategies.insert(String::from("default_strategy_3"), default_strategy_3);

        AppState {
            loaded_identity: None,
            loaded_wallet: None,
            known_identities: BTreeMap::new(),
            known_contracts,
            available_strategies,
            current_strategy: None,
            selected_strategy: None,
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
struct AppStateInSerializationFormat {
    pub loaded_identity : Option<Identity>,
    pub loaded_wallet: Option<Wallet>,
    pub known_identities: BTreeMap<String, Identity>,
    pub known_contracts: BTreeMap<String, Vec<u8>>,
    pub available_strategies: BTreeMap<String, Vec<u8>>,
    pub current_strategy: Option<String>,
    pub selected_strategy: Option<String>,
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
            loaded_identity, loaded_wallet, known_identities, known_contracts, available_strategies, current_strategy, selected_strategy
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
                let serialized_strategy = strategy.strategy.serialize_consume_to_bytes_with_platform_version(platform_version)?;
                Ok((key, serialized_strategy))
            })
            .collect::<Result<BTreeMap<String, Vec<u8>>, ProtocolError>>()?;

        let app_state_in_serialization_format = AppStateInSerializationFormat {
            loaded_identity,
            loaded_wallet: loaded_wallet.map(|wallet| wallet.deref().clone()),
            known_identities,
            known_contracts: known_contracts_in_serialization_format,
            available_strategies: available_strategies_in_serialization_format,
            current_strategy,
            selected_strategy,
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
            loaded_identity, loaded_wallet, known_identities, known_contracts, available_strategies, current_strategy, selected_strategy
        } = app_state;

        let known_contracts = known_contracts
            .into_iter()
            .map(|(key, contract)| {
                let contract = CreatedDataContract::versioned_deserialize(contract.as_slice(), validate, platform_version)?;
                Ok((key, contract))
            })
            .collect::<Result<BTreeMap<String, CreatedDataContract>, ProtocolError>>()?;

        let available_strategies = available_strategies
            .into_iter()
            .map(|(key, strategy)| {
                let strategy = StrategyDetails::versioned_deserialize(strategy.as_slice(), validate, platform_version)?;
                Ok((key, strategy))
            })
            .collect::<Result<BTreeMap<String, StrategyDetails>, ProtocolError>>()?;

        Ok(AppState {
            loaded_identity,
            loaded_wallet: loaded_wallet.map(|loaded_wallet| Arc::new(loaded_wallet)),
            known_identities,
            known_contracts,
            available_strategies,
            current_strategy,
            selected_strategy,
        })
    }
}

impl AppState {
    pub fn load() -> AppState {
        let path = Path::new("explorer.state");

        let Ok(read_result) = fs::read(path) else {
            return AppState::default()
        };

        let Ok(app_state) = AppState::versioned_deserialize(read_result.as_slice(), false, PlatformVersion::get(CURRENT_PROTOCOL_VERSION).unwrap()) else {
            return AppState::default()
        };

        if let Some(wallet) = app_state.loaded_wallet.as_ref() {
            let wallet = wallet.clone();
            task::spawn(async move {
                let _ = wallet.reload_utxos().await;
            });
        }

        app_state
    }

    pub fn save(&self) {
        let platform_version = PlatformVersion::get(CURRENT_PROTOCOL_VERSION).unwrap();
        let path = Path::new("explorer.state");

        let serialized_state = self.serialize_to_bytes_with_platform_version(platform_version).expect("expected to save state");
        fs::write(path, serialized_state).unwrap();
    }
}