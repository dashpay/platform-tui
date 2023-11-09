//! Strategy stuff
//!

use std::{collections::BTreeMap, fs, path::Path};

use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, created_data_contract::CreatedDataContract,
    },
    platform_value::string_encoding::Encoding,
    prelude::{DataContract, Identity},
};
use strategy_tests::{
    frequency::Frequency,
    operations::{DataContractUpdateOp, DocumentAction, IdentityUpdateOp, OperationType},
    Strategy,
};

pub fn default_strategy() -> Strategy {
    Strategy {
        contracts_with_updates: vec![],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        signer: None,
    }
}

pub trait Description {
    fn strategy_description(&self) -> BTreeMap<String, String>;
    fn id_to_name(id: &str) -> Option<String>;
}

impl Description for Strategy {
    fn id_to_name(id: &str) -> Option<String> {
        let dir = Path::new("supporting_files/contract");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.path().extension()? == "json" {
                        let content = fs::read_to_string(entry.path()).ok()?;
                        let json_content: serde_json::Value =
                            serde_json::from_str(&content).ok()?;
                        if json_content["id"].as_str() == Some(id) {
                            return Some(entry.path().file_stem()?.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
        None
    }

    fn strategy_description(&self) -> BTreeMap<String, String> {
        let mut desc = BTreeMap::new();

        desc.insert(
            "contracts_with_updates".to_string(),
            self.contracts_with_updates
                .iter()
                .map(|(contract, updates)| {
                    let contract_name = match contract {
                        CreatedDataContract::V0(v0) => match &v0.data_contract {
                            DataContract::V0(dc_v0) => {
                                Self::id_to_name(&dc_v0.id().to_string(Encoding::Base58))
                            }
                        },
                    }
                    .unwrap_or_else(|| "Unknown".to_string()); // use "Unknown" if no name found

                    let updates_names = updates.as_ref().map_or("".to_string(), |map| {
                        map.values()
                            .filter_map(|update_contract| match update_contract {
                                CreatedDataContract::V0(v0_update) => {
                                    match &v0_update.data_contract {
                                        DataContract::V0(dc_v0_update) => Self::id_to_name(
                                            &dc_v0_update.id().to_string(Encoding::Base58),
                                        ),
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("::")
                    });
                    format!(
                        "{}{}",
                        contract_name,
                        if updates_names.is_empty() {
                            "".to_string()
                        } else {
                            format!("::{}", updates_names)
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        );

        desc.insert(
            "operations".to_string(),
            self.operations
                .iter()
                .map(|operation| {
                    let op_type_str = match &operation.op_type {
                        OperationType::Document(doc_op) => {
                            let action_str = match &doc_op.action {
                                DocumentAction::DocumentActionInsertRandom(_, _) => "InsertRandom",
                                DocumentAction::DocumentActionDelete => "Delete",
                                DocumentAction::DocumentActionReplace => "Replace",
                                DocumentAction::DocumentActionInsertSpecific(_, _, _, _) => {
                                    "InsertSpecific"
                                }
                            };
                            format!("DocumentAction::{}", action_str)
                        }
                        OperationType::IdentityTopUp => "IdentityTopUp".to_string(),
                        OperationType::IdentityUpdate(update_type) => match update_type {
                            IdentityUpdateOp::IdentityUpdateAddKeys(num) => {
                                format!("IdentityUpdate::AddKeys::{}", num)
                            }
                            IdentityUpdateOp::IdentityUpdateDisableKey(num) => {
                                format!("IdentityUpdate::DisableKey::{}", num)
                            }
                        },
                        OperationType::IdentityWithdrawal => "IdentityWithdrawal".to_string(),
                        OperationType::ContractCreate(_, _) => "ContractCreate".to_string(),
                        OperationType::ContractUpdate(data_contract_update_op) => {
                            match data_contract_update_op {
                                DataContractUpdateOp::DataContractNewDocumentTypes(_) => {
                                    "ContractUpdate::NewDocTypes".to_string()
                                }
                                DataContractUpdateOp::DataContractNewOptionalFields(_, _) => {
                                    "ContractUpdate::NewFields".to_string()
                                }
                            }
                        }
                        OperationType::IdentityTransfer => "IdentityTransfer".to_string(),
                    };
                    let frequency_str = format!(
                        "TPB={}::CPB={}",
                        operation.frequency.times_per_block_range.start,
                        operation
                            .frequency
                            .chance_per_block
                            .map_or("None".to_string(), |chance| format!("{:.2}", chance)),
                    );
                    format!("{}::{}", op_type_str, frequency_str)
                })
                .collect::<Vec<_>>()
                .join("; "),
        );

        let start_identities_description =
            if let Some((first_identity_enum, _)) = self.start_identities.first() {
                let num_identities = self.start_identities.len();
                if num_identities > 0 {
                    let num_keys = match first_identity_enum {
                        Identity::V0(identity_v0) => identity_v0.public_keys.len(),
                    };
                    format!("Identities={}::Keys={}", num_identities, num_keys)
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };
        desc.insert("start_identities".to_string(), start_identities_description);

        let insert_description = if self.identities_inserts.times_per_block_range.end > 0 {
            format!(
                "TPB={}::CPB={}",
                self.identities_inserts.times_per_block_range.start,
                self.identities_inserts
                    .chance_per_block
                    .map_or("None".to_string(), |chance| format!("{:.2}", chance))
            )
        } else {
            "".to_string()
        };
        desc.insert("identities_inserts".to_string(), insert_description);

        desc
    }
}
