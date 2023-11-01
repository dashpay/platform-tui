//! Strategy stuff
//! 

use std::collections::BTreeMap;

use dpp::{data_contract::{created_data_contract::CreatedDataContract, accessors::v0::DataContractV0Getters}, prelude::{DataContract, Identity}, platform_value::string_encoding::Encoding};
use strategy_tests::{Strategy, frequency::Frequency, operations::{OperationType, DocumentAction}};

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
}

impl Description for Strategy {
    fn strategy_description(&self) -> BTreeMap<String, String> {
        let mut desc = BTreeMap::new();

        desc.insert(
            "contracts_with_updates".to_string(),
            self.contracts_with_updates
                .iter()
                .map(|(contract, updates)| {
                    let contract_id = match contract {
                        CreatedDataContract::V0(v0) => match &v0.data_contract {
                            DataContract::V0(dc_v0) => dc_v0.id().to_string(Encoding::Base58),
                        },
                    };
                    let updates_ids = updates
                        .as_ref()
                        .map_or("".to_string(), |map| {
                            map.values()
                                .map(|update_contract| {
                                    match update_contract {
                                        CreatedDataContract::V0(v0_update) => match &v0_update.data_contract {
                                            DataContract::V0(dc_v0_update) => dc_v0_update.id().to_string(Encoding::Base58),
                                        },
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("::")
                        });
                    format!("{}{}", contract_id, if updates_ids.is_empty() { "".to_string() } else { format!("::{}", updates_ids) })
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
                                DocumentAction::DocumentActionInsertRandom(_, _) => "DocumentActionInsertRandom",
                                DocumentAction::DocumentActionDelete => "DocumentActionDelete",
                                DocumentAction::DocumentActionReplace => "DocumentActionReplace",
                                DocumentAction::DocumentActionInsertSpecific(_, _, _, _) => "DocumentActionInsertSpecific",
                            };
                            format!("Document::{}", action_str)
                        },
                        OperationType::IdentityTopUp => "IdentityTopUp".to_string(),
                        OperationType::IdentityUpdate(_) => "IdentityUpdate".to_string(),
                        OperationType::IdentityWithdrawal => "IdentityWithdrawal".to_string(),
                        OperationType::ContractCreate(_, _) => "ContractCreate".to_string(),
                        OperationType::ContractUpdate(_) => "ContractUpdate".to_string(),
                        OperationType::IdentityTransfer => "IdentityTransfer".to_string(),
                    };
                    let frequency_str = format!(
                        "TPBR{}::CPB{}",
                        operation.frequency.times_per_block_range.end,
                        operation.frequency.chance_per_block.map_or("None".to_string(), |chance| format!("{:.2}", chance)),
                    );
                    format!("{}::{}", op_type_str, frequency_str)
                })
                .collect::<Vec<_>>()
                .join("; "),
        );

        if let Some((first_identity_enum, _)) = self.start_identities.first() {
            let num_identities = self.start_identities.len();
            let num_keys = match first_identity_enum {
                Identity::V0(identity_v0) => identity_v0.public_keys.len(),
                // Add more variants as they're defined
            };
            desc.insert(
                "start_identities".to_string(),
                format!("Identities={}::Keys={}", num_identities, num_keys),
            );
        } else {
            // Handle the case where start_identities is empty if needed
            desc.insert("start_identities".to_string(), "Identities=0::Keys=0".to_string());
        }

        desc.insert(
            "identities_inserts".to_string(),
            format!(
                "TPBR={}::CPB={}",
                self.identities_inserts.times_per_block_range.end,
                self.identities_inserts.chance_per_block.map_or("None".to_string(), |chance| format!("{:.2}", chance))
            ),
        );

        desc
    }
}
