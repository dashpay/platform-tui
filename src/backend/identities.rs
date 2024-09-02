//! Identities backend logic.

use chrono::Utc;
use dashcore::hashes::Hash;
use std::io::Write;
use std::{
    collections::{BTreeMap, HashSet},
    fs::OpenOptions,
    time::Duration,
};

use dapi_grpc::{
    core::v0::{
        BroadcastTransactionRequest, GetBlockchainStatusRequest, GetTransactionRequest,
        GetTransactionResponse,
    },
    platform::v0::{
        get_identity_balance_request::{self, GetIdentityBalanceRequestV0},
        GetIdentityBalanceRequest,
    },
};
use dash_sdk::{
    platform::{
        transition::{
            broadcast::BroadcastStateTransition, put_document::PutDocument,
            put_identity::PutIdentity, put_settings::PutSettings, top_up_identity::TopUpIdentity,
            withdraw_from_identity::WithdrawFromIdentity,
        },
        Fetch,
    },
    Sdk,
};
use dpp::{
    dashcore::Network,
    data_contract::DataContract,
    identity::{hash::IdentityPublicKeyHashMethodsV0, KeyID},
};
use dpp::{
    dashcore::{self, key::Secp256k1},
    identity::SecurityLevel,
};
use dpp::{
    dashcore::{psbt::serialize::Serialize, Address, PrivateKey, Transaction},
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        },
    },
    data_contracts::dpns_contract,
    document::{DocumentV0Getters, DocumentV0Setters},
    identity::{
        accessors::{IdentityGettersV0, IdentitySettersV0},
        identity_public_key::{accessors::v0::IdentityPublicKeyGettersV0, v0::IdentityPublicKeyV0},
        KeyType, PartialIdentity, Purpose as KeyPurpose, SecurityLevel as KeySecurityLevel,
    },
    platform_value::{string_encoding::Encoding, Bytes32, Identifier},
    prelude::{AssetLockProof, Identity, IdentityPublicKey},
    state_transition::{
        documents_batch_transition::{
            methods::v0::DocumentsBatchTransitionMethodsV0, DocumentsBatchTransition,
        },
        identity_credit_transfer_transition::{
            accessors::IdentityCreditTransferTransitionAccessorsV0,
            IdentityCreditTransferTransition,
        },
        identity_update_transition::{
            methods::IdentityUpdateTransitionMethodsV0, v0::IdentityUpdateTransitionV0,
        },
        proof_result::StateTransitionProofResult,
        public_key_in_creation::v0::IdentityPublicKeyInCreationV0,
        StateTransition,
    },
    util::{hash::hash_double, strings::convert_to_homograph_safe_chars},
    version::PlatformVersion,
};
use dpp::{identity::Purpose, ProtocolError};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
use sha2::{Digest, Sha256};
use simple_signer::signer::SimpleSigner;
use tokio::sync::{MappedMutexGuard, MutexGuard};

use super::{
    insight::InsightError, set_clipboard, state::IdentityPrivateKeysMap, wallet::WalletError,
    AppStateUpdate, CompletedTaskPayload, Wallet,
};
use crate::backend::{error::Error, stringify_result_keep_item, AppState, BackendEvent, Task};

pub(super) async fn fetch_identity_by_b58_id(
    sdk: &Sdk,
    base58_id: &str,
) -> Result<(Option<Identity>, String), String> {
    let id_bytes = Identifier::from_string(base58_id, Encoding::Base58)
        .map_err(|_| "Can't parse identifier as base58 string".to_owned())?;

    let fetch_result = Identity::fetch(sdk, id_bytes).await;
    stringify_result_keep_item(fetch_result)
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdentityTask {
    RegisterIdentity(u64),
    LoadKnownIdentity(Identifier),
    ContinueRegisteringIdentity,
    TopUpIdentity(u64),
    WithdrawFromIdentity(u64),
    Refresh,
    RefreshAllKnown,
    CopyIdentityId,
    AddIdentityKey {
        key_type: KeyType,
        security_level: KeySecurityLevel,
        purpose: KeyPurpose,
    },
    ClearLoadedIdentity,
    ClearRegistrationOfIdentityInProgress,
    TransferCredits(String, f64),
    LoadMasternodeIdentity(String, String),
    LoadIdentityById(String),
    AddPrivateKeys(Vec<String>),
    ForgetIdentity(Identifier),
    RegisterDPNSName(Identity, String),
}

impl AppState {
    pub async fn run_identity_task(&self, sdk: &Sdk, task: IdentityTask) -> BackendEvent {
        match task {
            IdentityTask::RegisterIdentity(amount) => {
                let result = self.register_new_identity(sdk, amount).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Executed successfully.\n\nPrivate keys were logged to supporting_files/new_identity_private_keys.log\n\nIt's recommended that you copy this file to a safe place so you don't lost your funds.".into())
                    .map_err(|e| e.to_string());
                let app_state_update = match result {
                    Ok(identity) => AppStateUpdate::LoadedIdentity(identity),
                    Err(_) => AppStateUpdate::IdentityRegistrationProgressed,
                };

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result,
                    app_state_update,
                }
            }
            IdentityTask::LoadKnownIdentity(ref identifier) => {
                let private_key_map = self.known_identities_private_keys.lock().await;

                // Check if any key in the map matches the identifier, regardless of the key ID
                let has_identifier = private_key_map.keys().any(|(id, _)| *id == identifier);

                if !has_identifier {
                    return BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(format!(
                            "No private keys known for this identity. Try loading it with the private keys."
                        )),
                    };
                }

                match Identity::fetch(&sdk, *identifier).await {
                    Ok(new_identity) => {
                        let mut loaded_identity_lock = self.loaded_identity.lock().await;
                        *loaded_identity_lock = new_identity;
                        BackendEvent::TaskCompletedStateChange {
                            task: Task::Identity(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                "Successfully loaded identity".to_owned(),
                            )),
                            app_state_update: AppStateUpdate::LoadedIdentity(MutexGuard::map(
                                loaded_identity_lock,
                                |identity| identity.as_mut().expect("checked above"),
                            )),
                        }
                    }
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(e.to_string()),
                    },
                }
            }
            IdentityTask::ContinueRegisteringIdentity => {
                let result = self.register_new_identity(sdk, 0).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Executed successfully".into())
                    .map_err(|e| e.to_string());
                let app_state_update = match result {
                    Ok(identity) => AppStateUpdate::LoadedIdentity(identity),
                    Err(_) => AppStateUpdate::IdentityRegistrationProgressed,
                };

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result,
                    app_state_update,
                }
            }
            IdentityTask::ClearRegistrationOfIdentityInProgress => {
                let mut loaded_identity_asset_lock_private_key_in_creation = self
                    .identity_asset_lock_private_key_in_creation
                    .lock()
                    .await;
                *loaded_identity_asset_lock_private_key_in_creation = None;
                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result: Ok(CompletedTaskPayload::String(
                        "Cleared registration of identity in progress".to_string(),
                    )),
                    app_state_update: AppStateUpdate::ClearedLoadedIdentity,
                }
            }
            IdentityTask::ClearLoadedIdentity => {
                let mut loaded_identity = self.loaded_identity.lock().await;
                *loaded_identity = None;
                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result: Ok(CompletedTaskPayload::String(
                        "Cleared loaded identity".to_string(),
                    )),
                    app_state_update: AppStateUpdate::ClearedLoadedIdentity,
                }
            }
            IdentityTask::Refresh => {
                let result = self.refresh_loaded_identity(sdk).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Executed successfully".into())
                    .map_err(|e| e.to_string());
                let app_state_update = match result {
                    Ok(identity) => AppStateUpdate::LoadedIdentity(identity),
                    Err(_) => AppStateUpdate::IdentityRegistrationProgressed,
                };

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result,
                    app_state_update,
                }
            }
            IdentityTask::RefreshAllKnown => {
                let result = self.refresh_all_known_identities(sdk).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Executed successfully".into())
                    .map_err(|e| e.to_string());
                let app_state_update = match result {
                    Ok(identities) => AppStateUpdate::KnownIdentities(identities),
                    Err(_) => AppStateUpdate::IdentityRegistrationProgressed,
                };

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result,
                    app_state_update,
                }
            }
            IdentityTask::TopUpIdentity(amount) => {
                let result = self.top_up_identity(sdk, amount).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Top up success".into())
                    .map_err(|e| e.to_string());
                match result {
                    Ok(identity) => BackendEvent::TaskCompletedStateChange {
                        task: Task::Identity(task),
                        execution_result,
                        app_state_update: AppStateUpdate::LoadedIdentity(identity),
                    },
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(e.to_string()),
                    },
                }
            }
            IdentityTask::WithdrawFromIdentity(amount) => {
                let result = self.withdraw_from_identity(sdk, amount).await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "Successful withdrawal".into())
                    .map_err(|e| e.to_string());
                match result {
                    Ok(identity) => BackendEvent::TaskCompletedStateChange {
                        task: Task::Identity(task),
                        execution_result,
                        app_state_update: AppStateUpdate::LoadedIdentity(identity),
                    },
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(e.to_string()),
                    },
                }
            }
            IdentityTask::CopyIdentityId => {
                if let Some(loaded_identity) = self.loaded_identity.lock().await.as_ref() {
                    let id = loaded_identity.id();
                    if set_clipboard(id.to_string(Encoding::Base58)).await.is_ok() {
                        BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Ok("Copied Identity Id".into()),
                        }
                    } else {
                        BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err("Clipboard is not supported".into()),
                        }
                    }
                } else {
                    BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err("Failed to copy Identity Id".into()),
                    }
                }
            }
            IdentityTask::AddIdentityKey {
                key_type,
                security_level,
                purpose,
            } => {
                let loaded_identity_lock = self.loaded_identity.lock().await;
                let loaded_identity = if loaded_identity_lock.is_some() {
                    MutexGuard::map(loaded_identity_lock, |identity| {
                        identity.as_mut().expect("checked above")
                    })
                } else {
                    return BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err("No identity loaded".to_owned()),
                    };
                };

                let identity_private_keys_lock = self.known_identities_private_keys.lock().await;
                match add_identity_key(
                    sdk,
                    loaded_identity,
                    identity_private_keys_lock,
                    key_type,
                    security_level,
                    purpose,
                )
                .await
                {
                    Ok(app_state_update) => BackendEvent::TaskCompletedStateChange {
                        task: Task::Identity(task),
                        execution_result: Ok(CompletedTaskPayload::String(
                            "Successfully added a key to the identity".to_owned(),
                        )),
                        app_state_update,
                    },
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(e),
                    },
                }
            }
            IdentityTask::TransferCredits(ref recipient, amount) => {
                let recipient_id = match Identifier::from_string(&recipient, Encoding::Base58) {
                    Ok(id) => id,
                    Err(_) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                "Can't parse identifier as base58 string".to_string(),
                            )),
                        }
                    }
                };
                let mut transfer_transition =
                    IdentityCreditTransferTransition::default_versioned(sdk.version())
                        .expect("Expected to create a default credit transfer transition");
                transfer_transition.set_amount((amount * 100_000_000_000.0) as u64);
                transfer_transition.set_recipient_id(recipient_id);
                let loaded_identity = self.loaded_identity.lock().await;
                if let Some(identity) = loaded_identity.as_ref() {
                    transfer_transition.set_identity_id(identity.id());
                    let nonce = match sdk.get_identity_nonce(identity.id(), true, None).await {
                        Ok(nonce) => nonce,
                        Err(e) => {
                            return BackendEvent::TaskCompleted {
                                task: Task::Identity(task),
                                execution_result: Err(format!(
                                    "Failed to get nonce from Platform: {e}"
                                )),
                            }
                        }
                    };
                    transfer_transition.set_nonce(nonce);

                    let mut transition =
                        StateTransition::IdentityCreditTransfer(transfer_transition);

                    let Some(identity_public_key) = identity.get_first_public_key_matching(
                        Purpose::TRANSFER,
                        HashSet::from([SecurityLevel::CRITICAL]),
                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                        false,
                    ) else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err("No public key for transfer".to_string()),
                        };
                    };

                    let loaded_identity_private_keys =
                        self.known_identities_private_keys.lock().await;
                    let Some(private_key) = loaded_identity_private_keys
                        .get(&(identity.id(), identity_public_key.id()))
                    else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err("No private key for transfer".to_string()),
                        };
                    };

                    let mut signer = SimpleSigner::default();

                    signer.add_key(identity_public_key.clone(), private_key.to_vec());

                    if let Err(e) = transition.sign_external(
                        identity_public_key,
                        &signer,
                        None::<fn(Identifier, String) -> Result<SecurityLevel, ProtocolError>>,
                    ) {
                        BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err(e.to_string()),
                        }
                    } else {
                        match transition.broadcast_and_wait(sdk, None).await {
                            Ok(_) => BackendEvent::TaskCompletedStateChange {
                                task: Task::Identity(task),
                                execution_result: Ok(CompletedTaskPayload::String(
                                    "Credit transfer successful.".to_owned(),
                                )),
                                app_state_update: AppStateUpdate::IdentityCreditsTransferred,
                            },
                            Err(e) => BackendEvent::TaskCompleted {
                                task: Task::Identity(task),
                                execution_result: Err(e.to_string()),
                            },
                        }
                    }
                } else {
                    BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Ok(CompletedTaskPayload::String(
                            "No loaded identity for credit transfer".to_string(),
                        )),
                    }
                }
            }
            IdentityTask::LoadMasternodeIdentity(ref pro_tx_hash, ref private_key_in_wif) => {
                // Convert proTxHash to bytes
                let pro_tx_hash_bytes = match hex::decode(pro_tx_hash) {
                    Ok(hash) => hash,
                    Err(e) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err(format!(
                                "Failed to decode proTxHash from hex: {}",
                                e
                            )),
                        };
                    }
                };

                // Get the address from the private key
                let private_key = match PrivateKey::from_wif(private_key_in_wif) {
                    Ok(key) => key,
                    Err(e) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Identity(task),
                            execution_result: Err(format!(
                                "Failed to convert private key from WIF: {}",
                                e
                            )),
                        };
                    }
                };
                let public_key = private_key.public_key(&Secp256k1::new());
                let pubkey_hash = public_key.pubkey_hash();
                let address = pubkey_hash.as_byte_array();

                // Hash address with proTxHash to get identity id of the identity
                let mut hasher = Sha256::new();
                hasher.update(pro_tx_hash_bytes.clone());
                hasher.update(address);
                let identity_id = hasher.finalize();

                // Convert to bs58
                let identity_id_bs58 = bs58::encode(identity_id).into_string();

                // Fetch the identity from Platform
                let result = fetch_identity_by_b58_id(sdk, &identity_id_bs58).await;
                match result {
                    Ok(evonode_identity_option) => {
                        if let Some(evonode_identity) = evonode_identity_option.0 {
                            // Get the IdentityPublicKey from Platform
                            // This is necessary because we need the id, which PublicKey struct doesn't have
                            let fetched_voting_public_key_result = evonode_identity
                                .get_first_public_key_matching(
                                    Purpose::VOTING,
                                    SecurityLevel::full_range().into(),
                                    KeyType::all_key_types().into(),
                                    false,
                                );

                            let fetched_voting_public_key = match fetched_voting_public_key_result {
                                Some(key) => key,
                                None => return BackendEvent::TaskCompleted {
                                    task: Task::Identity(task),
                                    execution_result: Err(format!("No voting key found (only voting Evonode identities are currently supported)")),
                                }
                            };

                            // Insert private key into the state for later use
                            let mut identity_private_keys =
                                self.known_identities_private_keys.lock().await;
                            identity_private_keys.insert(
                                (evonode_identity.id(), fetched_voting_public_key.id()),
                                private_key.to_bytes(),
                            );

                            // Insert into known_identities
                            let mut known_identities_lock = self.known_identities.lock().await;
                            known_identities_lock
                                .insert(evonode_identity.id(), evonode_identity.clone());

                            // Set loaded identity
                            let mut loaded_identity = self.loaded_identity.lock().await;
                            loaded_identity.replace(evonode_identity);

                            // Store proTxHash in AppState
                            let mut pro_tx_hash_lock =
                                self.loaded_identity_pro_tx_hash.lock().await;
                            pro_tx_hash_lock.replace(
                                Identifier::from_bytes(&pro_tx_hash_bytes)
                                    .expect("Expected to get Identifier from proTxHash bytes"),
                            );

                            // Return BackendEvent
                            BackendEvent::TaskCompletedStateChange {
                                task: Task::Identity(task),
                                execution_result: Ok(CompletedTaskPayload::String(
                                    "Loaded Evonode Identity".to_string(),
                                )),
                                app_state_update: AppStateUpdate::LoadedEvonodeIdentity(
                                    MutexGuard::map(loaded_identity, |x| {
                                        x.as_mut().expect("assigned above")
                                    }),
                                ),
                            }
                        } else {
                            BackendEvent::TaskCompleted {
                                task: Task::Identity(task),
                                execution_result: Err(format!("No identity found")),
                            }
                        }
                    }
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(format!("{e}")),
                    },
                }
            }
            IdentityTask::RegisterDPNSName(ref identity, ref name) => {
                let identity_id = identity.id();

                let result = self
                    .register_dpns_name(sdk, identity, &identity_id, name)
                    .await;
                let execution_result = result
                    .as_ref()
                    .map(|_| "DPNS name registration successful".into())
                    .map_err(|e| e.to_string());
                let app_state_update = match result {
                    Ok(_) => {
                        // Add the username to the map of known identities to usernames
                        let mut names_map = self.known_identities_names.lock().await;
                        names_map
                            .entry(identity_id.clone())
                            .or_insert_with(Vec::new)
                            .push(
                                name.to_lowercase()
                                    .replace('l', "1")
                                    .replace('o', "0")
                                    .to_string(),
                            );

                        AppStateUpdate::DPNSNameRegistered(name.clone())
                    }
                    Err(e) => AppStateUpdate::DPNSNameRegistrationFailed(e.to_string()),
                };

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result,
                    app_state_update,
                }
            }
            IdentityTask::AddPrivateKeys(ref private_key_strings) => {
                let loaded_identity_lock = self.loaded_identity.lock().await;
                let identity_id = &loaded_identity_lock
                    .clone()
                    .expect("An identity should be loaded")
                    .id();
                drop(loaded_identity_lock);

                let result = Self::add_identity_with_private_keys_as_strings(
                    &self,
                    identity_id,
                    &private_key_strings,
                    sdk,
                )
                .await;

                if let Err(e) = result {
                    return BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(format!(
                            "Failed to add identity with private keys: {e}"
                        )),
                    };
                }

                let loaded_identity_lock = self.loaded_identity.lock().await;
                let loaded_identity_update = MutexGuard::map(loaded_identity_lock, |opt| {
                    opt.as_mut().expect("identity was set above")
                });

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(task),
                    execution_result: Ok("Added identity".into()),
                    app_state_update: AppStateUpdate::LoadedIdentity(loaded_identity_update),
                }
            }
            IdentityTask::LoadIdentityById(ref identity_id_string) => {
                let identity_id =
                    match Identifier::from_string(identity_id_string, Encoding::Base58) {
                        Ok(id) => id,
                        Err(e) => {
                            return BackendEvent::TaskCompleted {
                                task: Task::Identity(task),
                                execution_result: Err(format!(
                                    "Error converting base58 string to Identifier: {}",
                                    e
                                )),
                            };
                        }
                    };
                let identity_fetch_result = Identity::fetch_by_identifier(sdk, identity_id).await;
                match identity_fetch_result {
                    Ok(Some(identity)) => {
                        let mut loaded_identity_lock = self.loaded_identity.lock().await;
                        loaded_identity_lock.replace(identity.clone());
                        let loaded_identity_update = MutexGuard::map(loaded_identity_lock, |opt| {
                            opt.as_mut().expect("identity was set above")
                        });
                        let mut known_identities_lock = self.known_identities.lock().await;
                        known_identities_lock.insert(identity_id, identity);
                        BackendEvent::TaskCompletedStateChange {
                            task: Task::Identity(task),
                            execution_result: Ok("Loaded identity from base58 id".into()),
                            app_state_update: AppStateUpdate::LoadedIdentity(
                                loaded_identity_update,
                            ),
                        }
                    }
                    Ok(None) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err("No identity found with that id".into()),
                    },
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Identity(task),
                        execution_result: Err(format!("Error fetching identity by id: {}", e)),
                    },
                }
            }
            IdentityTask::ForgetIdentity(identifier) => {
                // Remove from known_identities
                let mut known_identities = self.known_identities.lock().await;
                known_identities.remove(&identifier);

                // Remove from known_identities_private_keys
                let mut known_identities_private_keys =
                    self.known_identities_private_keys.lock().await;
                let keys_to_remove: Vec<(Identifier, KeyID)> = known_identities_private_keys
                    .keys()
                    .filter(|(id, _)| *id == identifier)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    known_identities_private_keys.remove(&key);
                }

                // If this is the loaded identity, remove it from there
                let mut loaded_identity = self.loaded_identity.lock().await;
                if loaded_identity.is_some() {
                    let some_loaded_identity =
                        loaded_identity.clone().expect("Expected a loaded identity");
                    if some_loaded_identity.id() == identifier {
                        *loaded_identity = None;
                    }
                }

                BackendEvent::TaskCompletedStateChange {
                    task: Task::Identity(IdentityTask::ForgetIdentity(identifier)),
                    execution_result: Ok(CompletedTaskPayload::String(
                        "Successfully removed identity from TUI state".to_string(),
                    )),
                    app_state_update: AppStateUpdate::ForgotIdentity,
                }
            }
        }
    }

    pub(crate) async fn register_dpns_name(
        &self,
        sdk: &Sdk,
        identity: &Identity,      // Identity to register the name for and sign tx
        _identifier: &Identifier, // Once contract names are enabled, we can use this field
        name: &str,
    ) -> Result<(), Error> {
        let mut rng = StdRng::from_entropy();
        let platform_version = PlatformVersion::latest();

        let dpns_contract = match DataContract::fetch(
            &sdk,
            Into::<Identifier>::into(dpns_contract::ID_BYTES),
        )
        .await
        {
            Ok(contract) => contract.unwrap(),
            Err(e) => return Err(Error::SdkError(e)),
        };

        // Add DPNS to known contracts in app state
        let mut known_contracts = self.known_contracts.lock().await;
        known_contracts
            .entry("DPNS".to_string())
            .or_insert(dpns_contract.clone());

        let preorder_document_type = dpns_contract
            .document_type_for_name("preorder")
            .map_err(|_| Error::DPNSError("DPNS preorder document type not found".to_string()))?;
        let domain_document_type = dpns_contract
            .document_type_for_name("domain")
            .map_err(|_| Error::DPNSError("DPNS domain document type not found".to_string()))?;

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut preorder_document = preorder_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                &platform_version,
            )?;
        let mut domain_document = domain_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                &platform_version,
            )?;

        let salt: [u8; 32] = rng.gen();
        let mut salted_domain_buffer: Vec<u8> = vec![];
        salted_domain_buffer.extend(salt);
        salted_domain_buffer.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());
        let salted_domain_hash = hash_double(salted_domain_buffer);

        preorder_document.set("saltedDomainHash", salted_domain_hash.into());
        domain_document.set("parentDomainName", "dash".into());
        domain_document.set("normalizedParentDomainName", "dash".into());
        domain_document.set("label", name.into());
        domain_document.set(
            "normalizedLabel",
            convert_to_homograph_safe_chars(name).into(),
        );
        domain_document.set("records.identity", domain_document.owner_id().into());
        domain_document.set("subdomainRules.allowSubdomains", false.into());
        domain_document.set("preorderSalt", salt.into());

        let identity_contract_nonce = match sdk
            .get_identity_contract_nonce(
                identity.id(),
                dpns_contract.id(),
                true,
                Some(PutSettings {
                    request_settings: RequestSettings::default(),
                    identity_nonce_stale_time_s: Some(0),
                    user_fee_increase: None,
                }),
            )
            .await
        {
            Ok(nonce) => nonce,
            Err(e) => return Err(Error::SdkError(e)),
        };

        // TODO this is used in strategy tests too. It should be a function.
        // Get signer from loaded_identity
        // Convert loaded_identity to SimpleSigner
        let identity_private_keys_lock = self.known_identities_private_keys.lock().await;
        let signer = {
            let mut new_signer = SimpleSigner::default();
            let Identity::V0(identity_v0) = &*identity;
            for (key_id, public_key) in &identity_v0.public_keys {
                let identity_key_tuple = (identity_v0.id, *key_id);
                if let Some(private_key_bytes) = identity_private_keys_lock.get(&identity_key_tuple)
                {
                    new_signer
                        .private_keys
                        .insert(public_key.clone(), private_key_bytes.clone());
                }
            }
            new_signer
        };
        drop(identity_private_keys_lock);

        let public_key =
            match identity.get_first_public_key_matching(
                Purpose::AUTHENTICATION,
                HashSet::from([SecurityLevel::CRITICAL]),
                HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                false,
            ) {
                Some(key) => key,
                None => return Err(Error::DPNSError(
                    "Identity doesn't have an authentication key for signing document transitions"
                        .to_string(),
                )),
            };

        let preorder_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                preorder_document.clone(),
                preorder_document_type,
                entropy.0,
                public_key,
                identity_contract_nonce,
                0,
                &signer,
                &platform_version,
                None,
                None,
                None,
            )?;

        let domain_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                domain_document.clone(),
                domain_document_type,
                entropy.0,
                identity
                    .get_first_public_key_matching(
                        Purpose::AUTHENTICATION,
                        HashSet::from([SecurityLevel::CRITICAL]),
                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                        false,
                    )
                    .expect("expected to get a signing key"),
                identity_contract_nonce + 1,
                0,
                &signer,
                &platform_version,
                None,
                None,
                None,
            )?;

        preorder_transition.broadcast(sdk).await?;

        let _preorder_document =
            match <dash_sdk::platform::Document as PutDocument<SimpleSigner>>::wait_for_response::<
                '_,
                '_,
                '_,
            >(
                &preorder_document,
                sdk,
                preorder_transition,
                dpns_contract.clone().into(),
            )
            .await
            {
                Ok(document) => document,
                Err(e) => {
                    return Err(Error::DPNSError(format!(
                        "Preorder document failed to process: {e}"
                    )));
                }
            };

        domain_transition.broadcast(sdk).await?;

        let _domain_document =
            match <dash_sdk::platform::Document as PutDocument<SimpleSigner>>::wait_for_response::<
                '_,
                '_,
                '_,
            >(
                &domain_document,
                sdk,
                domain_transition,
                dpns_contract.into(),
            )
            .await
            {
                Ok(document) => document,
                Err(e) => {
                    return Err(Error::DPNSError(format!(
                        "Domain document failed to process: {e}"
                    )));
                }
            };

        Ok(())
    }

    pub(crate) async fn refresh_loaded_identity<'s>(
        &'s self,
        sdk: &Sdk,
    ) -> Result<MappedMutexGuard<'s, Identity>, Error> {
        let mut loaded_identity = self.loaded_identity.lock().await;

        if let Some(identity) = loaded_identity.as_ref() {
            let refreshed_identity = Identity::fetch(sdk, identity.id()).await?;
            if let Some(refreshed_identity) = refreshed_identity {
                loaded_identity.replace(refreshed_identity.clone());

                let mut known_identities = self.known_identities.lock().await;
                known_identities
                    .entry(refreshed_identity.id())
                    .and_modify(|id| *id = refreshed_identity.clone())
                    .or_insert(refreshed_identity);
            }
        } else {
            return Err(Error::IdentityRefreshError(
                "No identity loaded".to_string(),
            ));
        }
        let identity_result =
            MutexGuard::map(loaded_identity, |x| x.as_mut().expect("assigned above"));
        Ok(identity_result)
    }

    pub(crate) async fn refresh_all_known_identities<'s>(
        &'s self,
        sdk: &Sdk,
    ) -> Result<MappedMutexGuard<'s, BTreeMap<Identifier, Identity>>, Error> {
        let mut known_identities = self.known_identities.lock().await;

        let mut identities_to_refresh = Vec::new();

        // Collect all known identity IDs to refresh
        for identity in known_identities.values() {
            identities_to_refresh.push(identity.id());
        }

        for identity_id in identities_to_refresh {
            if let Some(refreshed_identity) = Identity::fetch(sdk, identity_id).await? {
                known_identities
                    .entry(identity_id)
                    .and_modify(|id| *id = refreshed_identity.clone())
                    .or_insert(refreshed_identity);
            } else {
                tracing::error!("Failed to refresh identity with ID: {}", identity_id);
            }
        }

        let identity_result = MutexGuard::map(known_identities, |x| x);
        Ok(identity_result)
    }

    pub(crate) async fn refresh_loaded_identity_balance(&mut self, sdk: &Sdk) -> Result<(), Error> {
        if let Some(identity) = self.loaded_identity.blocking_lock().as_mut() {
            let balance = u64::fetch(
                sdk,
                GetIdentityBalanceRequest {
                    version: Some(get_identity_balance_request::Version::V0(
                        GetIdentityBalanceRequestV0 {
                            id: identity.id().to_vec(),
                            prove: true,
                        },
                    )),
                },
            )
            .await?;
            if let Some(balance) = balance {
                identity.set_balance(balance)
            }
        }
        Ok(())
    }

    pub(crate) async fn register_new_identity<'s>(
        &'s self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<MappedMutexGuard<'s, Identity>, Error> {
        // First we need to make the transaction from the wallet
        // We start by getting a lock on the wallet

        let mut loaded_wallet = self.loaded_wallet.lock().await;
        let Some(wallet) = loaded_wallet.as_mut() else {
            return Err(Error::IdentityRegistrationError(
                "No wallet loaded".to_string(),
            ));
        };

        //// Core steps

        let mut identity_asset_lock_private_key_in_creation = self
            .identity_asset_lock_private_key_in_creation
            .lock()
            .await;

        // We create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (
            asset_lock_transaction,
            asset_lock_proof_private_key,
            maybe_asset_lock_proof,
            maybe_identity_info,
        ) = if let Some((
            asset_lock_transaction,
            asset_lock_proof_private_key,
            maybe_asset_lock_proof,
            maybe_identity,
        )) = identity_asset_lock_private_key_in_creation.as_ref()
        {
            (
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key.clone(),
                maybe_asset_lock_proof.clone(),
                maybe_identity.clone(),
            )
        } else {
            let (asset_lock_transaction, asset_lock_proof_private_key) =
                wallet.asset_lock_transaction(None, amount)?;

            identity_asset_lock_private_key_in_creation.replace((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key,
                None,
                None,
            ));

            (
                asset_lock_transaction,
                asset_lock_proof_private_key,
                None,
                None,
            )
        };

        let asset_lock_proof = if let Some(asset_lock_proof) = maybe_asset_lock_proof {
            asset_lock_proof.clone()
        } else {
            let asset_lock = Self::broadcast_and_retrieve_asset_lock(
                sdk,
                &asset_lock_transaction,
                &wallet.receive_address(),
            )
            .await
            .map_err(|e| {
                Error::SdkExplainedError("broadcasting transaction failed".to_string(), e)
            })?;

            identity_asset_lock_private_key_in_creation.replace((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key,
                Some(asset_lock.clone()),
                None,
            ));

            asset_lock
        };

        //// Platform steps

        let (identity, keys): (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) =
            if let Some(identity_info) = maybe_identity_info {
                identity_info.clone()
            } else {
                let mut std_rng = StdRng::from_entropy();
                // Create a random identity with master key
                let (mut identity, mut keys): (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) =
                    Identity::random_identity_with_main_keys_with_private_key(
                        2,
                        &mut std_rng,
                        sdk.version(),
                    )?;

                // Add a critical key
                let (critical_key, critical_private_key) =
                    IdentityPublicKey::random_ecdsa_critical_level_authentication_key(
                        2,
                        None,
                        sdk.version(),
                    )?;
                identity.add_public_key(critical_key.clone());
                keys.insert(critical_key, critical_private_key);

                // Add a key for transfers
                let (transfer_key, transfer_private_key) =
                    IdentityPublicKey::random_key_with_known_attributes(
                        3,
                        &mut std_rng,
                        KeyPurpose::TRANSFER,
                        KeySecurityLevel::CRITICAL,
                        KeyType::ECDSA_SECP256K1,
                        None,
                        sdk.version(),
                    )?;
                identity.add_public_key(transfer_key.clone());
                keys.insert(transfer_key, transfer_private_key);

                identity.set_id(
                    asset_lock_proof
                        .create_identifier()
                        .expect("expected to create an identifier"),
                );

                identity_asset_lock_private_key_in_creation.replace((
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key,
                    Some(asset_lock_proof.clone()),
                    Some((identity.clone(), keys.clone())),
                ));

                (identity, keys)
            };

        let mut signer = SimpleSigner::default();

        signer.add_keys(keys.clone());

        let updated_identity = identity
            .put_to_platform_and_wait_for_response(
                sdk,
                asset_lock_proof.clone(),
                &asset_lock_proof_private_key,
                &signer,
            )
            .await?;

        if updated_identity.id() != identity.id() {
            panic!("identity ids don't match");
        }

        // Log the identity ID and the private keys to a file
        let _ = log_identity_keys(&identity, &keys)
            .map_err(|e| tracing::error!("Failed to log private keys: {e}"));

        let mut loaded_identity = self.loaded_identity.lock().await;

        loaded_identity.replace(identity.clone());
        let identity_result =
            MutexGuard::map(loaded_identity, |x| x.as_mut().expect("assigned above"));

        let keys = identity_asset_lock_private_key_in_creation
            .take()
            .expect("expected something to be in creation")
            .3
            .expect("expected an identity")
            .1
            .into_iter()
            .map(|(key, private_key)| ((identity.id(), key.id()), private_key));

        let mut identity_private_keys = self.known_identities_private_keys.lock().await;

        identity_private_keys.extend(keys);

        let mut known_identities_lock = self.known_identities.lock().await;
        known_identities_lock.insert(identity.id(), identity);

        Ok(identity_result)
    }

    pub(crate) async fn top_up_identity<'s>(
        &'s self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<MappedMutexGuard<'s, Identity>, Error> {
        // First we need to make the transaction from the wallet
        // We start by getting a lock on the wallet

        let mut loaded_wallet = self.loaded_wallet.lock().await;
        let Some(wallet) = loaded_wallet.as_mut() else {
            return Err(Error::IdentityRegistrationError(
                "No wallet loaded".to_string(),
            ));
        };

        let mut identity_lock = self.loaded_identity.lock().await;

        let Some(identity) = identity_lock.as_mut() else {
            return Err(Error::IdentityTopUpError("No identity loaded".to_string()));
        };

        //// Core steps

        let mut identity_asset_lock_private_key_in_top_up =
            self.identity_asset_lock_private_key_in_top_up.lock().await;

        // We create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (asset_lock_transaction, asset_lock_proof_private_key, maybe_asset_lock_proof) =
            if let Some((
                asset_lock_transaction,
                asset_lock_proof_private_key,
                maybe_asset_lock_proof,
            )) = identity_asset_lock_private_key_in_top_up.as_ref()
            {
                (
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key.clone(),
                    maybe_asset_lock_proof.clone(),
                )
            } else {
                let (asset_lock_transaction, asset_lock_proof_private_key) =
                    wallet.asset_lock_transaction(None, amount)?;

                identity_asset_lock_private_key_in_top_up.replace((
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key,
                    None,
                ));

                (asset_lock_transaction, asset_lock_proof_private_key, None)
            };

        let asset_lock_proof = if let Some(asset_lock_proof) = maybe_asset_lock_proof {
            asset_lock_proof.clone()
        } else {
            let asset_lock = Self::broadcast_and_retrieve_asset_lock(
                sdk,
                &asset_lock_transaction,
                &wallet.receive_address(),
            )
            .await
            .map_err(|e| {
                Error::SdkExplainedError("error broadcasting transaction".to_string(), e)
            })?;

            identity_asset_lock_private_key_in_top_up.replace((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key,
                Some(asset_lock.clone()),
            ));

            asset_lock
        };

        //// Platform steps

        match identity
            .top_up_identity(
                sdk,
                asset_lock_proof.clone(),
                &asset_lock_proof_private_key,
                None,
            )
            .await
        {
            Ok(updated_identity_balance) => {
                identity.set_balance(updated_identity_balance);
            }
            Err(dash_sdk::Error::DapiClientError(error_string)) => {
                //todo in the future, errors should be proved with a proof, even from tenderdash

                if error_string.contains("state transition already in chain")
                    || error_string.contains("already completely used")
                {
                    // This state transition already existed
                    tracing::info!("we are starting over as the previous top up already existed");
                    let (new_asset_lock_transaction, new_asset_lock_proof_private_key) =
                        wallet.asset_lock_transaction(None, amount)?;

                    identity_asset_lock_private_key_in_top_up.replace((
                        new_asset_lock_transaction.clone(),
                        new_asset_lock_proof_private_key,
                        None,
                    ));

                    let new_asset_lock_proof = Self::broadcast_and_retrieve_asset_lock(
                        sdk,
                        &new_asset_lock_transaction,
                        &wallet.receive_address(),
                    )
                    .await
                    .map_err(|e| {
                        Error::SdkExplainedError("error broadcasting transaction".to_string(), e)
                    })?;

                    identity_asset_lock_private_key_in_top_up.replace((
                        new_asset_lock_transaction.clone(),
                        new_asset_lock_proof_private_key,
                        Some(new_asset_lock_proof.clone()),
                    ));

                    identity
                        .top_up_identity(
                            sdk,
                            new_asset_lock_proof.clone(),
                            &new_asset_lock_proof_private_key,
                            None,
                        )
                        .await?;
                } else {
                    return Err(dash_sdk::Error::DapiClientError(error_string).into());
                }
            }
            Err(e) => return Err(e.into()),
        }

        identity_asset_lock_private_key_in_top_up.take(); // clear the top up

        Ok(MutexGuard::map(identity_lock, |identity| {
            identity.as_mut().expect("checked above")
        })) // TODO too long above, better to refactor this one
    }

    pub(crate) async fn withdraw_from_identity<'s>(
        &'s self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<MappedMutexGuard<'s, Identity>, Error> {
        // First we need to make the transaction from the wallet
        // We start by getting a lock on the wallet

        let mut loaded_wallet = self.loaded_wallet.lock().await;
        let Some(wallet) = loaded_wallet.as_mut() else {
            return Err(Error::IdentityRegistrationError(
                "No wallet loaded".to_string(),
            ));
        };

        let new_receive_address = wallet.receive_address();

        let mut identity_lock = self.loaded_identity.lock().await;
        let Some(identity) = identity_lock.as_mut() else {
            return Err(Error::IdentityTopUpError("No identity loaded".to_string()));
        };

        let identity_public_key = identity
            .get_first_public_key_matching(
                KeyPurpose::TRANSFER,
                KeySecurityLevel::full_range().into(),
                KeyType::all_key_types().into(),
                false,
            )
            .ok_or(Error::IdentityWithdrawalError(
                "no withdrawal public key".to_string(),
            ))?;

        let loaded_identity_private_keys = self.known_identities_private_keys.lock().await;
        let Some(private_key) =
            loaded_identity_private_keys.get(&(identity.id(), identity_public_key.id()))
        else {
            return Err(Error::IdentityTopUpError(
                "No private key for withdrawal".to_string(),
            ));
        };

        let mut signer = SimpleSigner::default();

        signer.add_key(identity_public_key.clone(), private_key.to_vec());

        //// Platform steps

        let updated_identity_balance = identity
            .withdraw(sdk, new_receive_address, amount, None, None, signer, None)
            .await?;

        identity.set_balance(updated_identity_balance);

        Ok(MutexGuard::map(identity_lock, |identity| {
            identity.as_mut().expect("checked above")
        })) // TODO
    }

    pub(crate) async fn broadcast_and_retrieve_asset_lock(
        sdk: &Sdk,
        asset_lock_transaction: &Transaction,
        address: &Address,
    ) -> Result<AssetLockProof, dash_sdk::Error> {
        let _span = tracing::debug_span!(
            "broadcast_and_retrieve_asset_lock",
            transaction_id = asset_lock_transaction.txid().to_string(),
        )
        .entered();

        let block_hash = sdk
            .execute(GetBlockchainStatusRequest {}, RequestSettings::default())
            .await?
            .chain
            .map(|chain| chain.best_block_hash)
            .ok_or_else(|| dash_sdk::Error::DapiClientError("missing `chain` field".to_owned()))?;

        tracing::debug!(
            "starting the stream from the tip block hash {}",
            hex::encode(&block_hash)
        );

        let mut asset_lock_stream = sdk
            .start_instant_send_lock_stream(block_hash, address)
            .await?;

        tracing::debug!("stream is started");

        // we need to broadcast the transaction to core
        let request = BroadcastTransactionRequest {
            transaction: asset_lock_transaction.serialize(), /* transaction but how to encode it
                                                              * as bytes?, */
            allow_high_fees: false,
            bypass_limits: false,
        };

        tracing::debug!("broadcast the transaction");

        match sdk.execute(request, RequestSettings::default()).await {
            Ok(_) => tracing::debug!("transaction is successfully broadcasted"),
            Err(error) if error.to_string().contains("AlreadyExists") => {
                // Transaction is already broadcasted. We need to restart the stream from a
                // block when it was mined

                tracing::warn!("transaction is already broadcasted");

                let GetTransactionResponse { block_hash, .. } = sdk
                    .execute(
                        GetTransactionRequest {
                            id: asset_lock_transaction.txid().to_string(),
                        },
                        RequestSettings::default(),
                    )
                    .await?;

                tracing::debug!(
                    "restarting the stream from the transaction minded block hash {}",
                    hex::encode(&block_hash)
                );

                asset_lock_stream = sdk
                    .start_instant_send_lock_stream(block_hash, address)
                    .await?;

                tracing::debug!("stream is started");
            }
            Err(error) => {
                tracing::error!("transaction broadcast failed: {error}");

                return Err(error.into());
            }
        };

        tracing::debug!("waiting for asset lock proof");

        sdk.wait_for_asset_lock_proof_for_transaction(
            asset_lock_stream,
            asset_lock_transaction,
            Some(Duration::from_secs(4 * 60)),
        )
        .await
    }

    pub async fn retrieve_asset_lock_proof(
        sdk: &Sdk,
        wallet: &mut Wallet,
        amount: u64,
    ) -> Result<(AssetLockProof, PrivateKey), Error> {
        // Create the wallet registration transaction
        let (asset_lock_transaction, asset_lock_proof_private_key) =
            wallet.asset_lock_transaction(None, amount).map_err(|e| {
                Error::WalletError(WalletError::Insight(InsightError(format!(
                    "Wallet transaction error: {}",
                    e
                ))))
            })?;

        // Broadcast the transaction and retrieve the asset lock proof
        match Self::broadcast_and_retrieve_asset_lock(
            sdk,
            &asset_lock_transaction,
            &wallet.receive_address(),
        )
        .await
        {
            Ok(proof) => Ok((proof, asset_lock_proof_private_key)),
            Err(e) => Err(Error::SdkError(e)),
        }
    }

    pub async fn add_identity_with_private_keys_as_strings<'s>(
        &self,
        identity_id: &Identifier,
        private_keys_as_strings: &Vec<String>,
        sdk: &Sdk,
    ) -> Result<(), WalletError> {
        let private_keys = private_keys_as_strings
            .iter()
            .map(|private_key| match private_key.len() {
                64 => {
                    // hex
                    let bytes = match hex::decode(private_key) {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            return Err(WalletError::Custom("Failed to decode hex".to_string()))
                        }
                    };
                    match PrivateKey::from_slice(bytes.as_slice(), Network::Dash) {
                        Ok(key) => Ok(key),
                        Err(_) => {
                            return Err(WalletError::Custom("Expected private key".to_string()))
                        }
                    }
                }
                51 | 52 => {
                    // wif
                    match PrivateKey::from_wif(private_key) {
                        Ok(key) => Ok(key),
                        Err(_) => return Err(WalletError::Custom("Expected WIF key".to_string())),
                    }
                }
                _ => {
                    return Err(WalletError::Custom(
                        "Private key can't be decoded from hex or wif".to_string(),
                    ));
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::add_identity_with_private_keys(&self, identity_id, private_keys, sdk).await
    }

    pub async fn add_identity_with_private_keys<'s>(
        &self,
        identity_id: &Identifier,
        private_keys: Vec<PrivateKey>,
        sdk: &Sdk,
    ) -> Result<(), WalletError> {
        let identity_fetch_result = Identity::fetch(sdk, *identity_id).await;
        let identity = match identity_fetch_result {
            Ok(Some(identity)) => identity,
            Ok(None) => {
                return Err(WalletError::Custom(
                    "No identity found with that ID".to_string(),
                ));
            }
            Err(e) => {
                return Err(WalletError::Custom(format!(
                    "Error fetching identity: {}",
                    e
                )));
            }
        };

        let mut loaded_identity_lock = self.loaded_identity.lock().await;
        *loaded_identity_lock = Some(identity.clone());

        let mut identity_private_keys_map_lock = self.known_identities_private_keys.lock().await;

        for private_key in private_keys.iter() {
            let secp = Secp256k1::new();
            let public_key_hash = private_key.public_key(&secp).pubkey_hash();
            let key_id = identity
                .public_keys()
                .iter()
                .filter_map(|(key_id, identity_public_key)| {
                    (identity_public_key
                        .public_key_hash()
                        .expect("Expected to get public key hash from public key")
                        == public_key_hash.to_byte_array())
                    .then_some(key_id)
                })
                .next();

            if let Some(key_id) = key_id {
                identity_private_keys_map_lock
                    .insert((*identity_id, *key_id), private_key.to_bytes().to_vec());
            } else {
                return Err(WalletError::Custom(
                    "Public key hash derived from input private key does not match any identity public key".to_string(),
                ));
            }
        }

        Ok(())
    }
}

async fn add_identity_key<'a>(
    sdk: &Sdk,
    mut loaded_identity: MappedMutexGuard<'a, Identity>,
    mut identity_private_keys: MutexGuard<'a, IdentityPrivateKeysMap>,
    key_type: KeyType,
    security_level: KeySecurityLevel,
    purpose: KeyPurpose,
) -> Result<AppStateUpdate<'a>, String> {
    let mut rng = StdRng::from_entropy();
    let platform_version = sdk.version();

    let (public_key, private_key) = key_type
        .random_public_and_private_key_data(&mut rng, &platform_version)
        .map_err(|e| format!("Cannot generate key pair: {e}"))?;
    let identity_public_key: IdentityPublicKey = IdentityPublicKeyV0 {
        id: loaded_identity.get_public_key_max_id() + 1,
        purpose,
        security_level,
        contract_bounds: None,
        key_type,
        read_only: false,
        data: public_key.into(),
        disabled_at: None,
    }
    .into();

    let (master_public_key_id, master_public_key) = loaded_identity
        .public_keys()
        .iter()
        .find(|(_, key)| key.security_level() == KeySecurityLevel::MASTER)
        .ok_or_else(|| "No master key found for identity".to_owned())?;
    let master_private_key = identity_private_keys
        .get(&(loaded_identity.id(), *master_public_key_id))
        .ok_or_else(|| "Master private key not found".to_owned())?;

    let mut signer = SimpleSigner::default();
    signer.add_key(master_public_key.clone(), master_private_key.to_vec());
    signer.add_key(identity_public_key.clone(), private_key.clone());

    let mut identity_updated = loaded_identity.clone();
    identity_updated.bump_revision();

    let new_identity_nonce = sdk
        .get_identity_nonce(identity_updated.id(), true, None)
        .await
        .map_err(|e| format!("Can't get new identity nonce: {e}"))?;

    let identity_update_transition = IdentityUpdateTransitionV0::try_from_identity_with_signer(
        &identity_updated,
        master_public_key_id,
        vec![Into::<IdentityPublicKeyInCreationV0>::into(identity_public_key.clone()).into()],
        Vec::new(),
        new_identity_nonce,
        0,
        &signer,
        &platform_version,
        None,
    )
    .map_err(|e| format!("Unable to create state transition: {e}"))?;

    let StateTransitionProofResult::VerifiedPartialIdentity(PartialIdentity {
        loaded_public_keys,
        balance: Some(balance),
        revision: Some(revision),
        ..
    }) = identity_update_transition
        .broadcast_and_wait(sdk, None)
        .await
        .map_err(|e| format!("Error broadcasting identity update transition: {e}"))?
    else {
        return Err(format!("Cannot verify identity update transition proof"));
    };

    loaded_identity.set_balance(balance);
    loaded_identity.set_revision(revision);
    loaded_identity.set_public_keys(loaded_public_keys);

    identity_private_keys.insert(
        (loaded_identity.id(), identity_public_key.id()),
        private_key.clone(),
    );

    // Log the newly added key
    let mut keys_to_log = BTreeMap::new();
    keys_to_log.insert(identity_public_key.clone(), private_key);
    log_identity_keys(&loaded_identity, &keys_to_log)
        .map_err(|e| format!("Failed to log identity keys: {e}"))?;

    Ok(AppStateUpdate::LoadedIdentity(loaded_identity))
}

fn log_identity_keys(
    identity: &Identity,
    keys: &BTreeMap<IdentityPublicKey, Vec<u8>>,
) -> std::io::Result<()> {
    // Open the log file in append mode
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("supporting_files/new_identity_private_keys.log")?;

    // Get the current date and time
    let now = Utc::now().to_rfc3339();

    // Log each key in the identity
    for (key, private_key) in keys {
        writeln!(
            file,
            "{}, Identity ID: {}, Public Key: {} ({}), Private Key: {:x?}",
            now,
            identity.id(),
            key.id(),
            key.purpose(),
            hex::encode(private_key)
        )?;
    }

    Ok(())
}
