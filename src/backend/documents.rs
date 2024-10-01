use chrono::{DateTime, Duration, Utc};
use dash_sdk::platform::transition::vote::PutVote;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::format,
    iter,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use dash_sdk::{
    platform::{
        transition::{
            purchase_document::PurchaseDocument, put_document::PutDocument,
            transfer_document::TransferDocument, update_price_of_document::UpdatePriceOfDocument,
        },
        DocumentQuery, FetchMany,
    },
    Sdk,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters,
            methods::DocumentTypeV0Methods,
            random_document::{CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType},
            DocumentType,
        },
    },
    document::{
        property_names::{
            CREATED_AT, CREATED_AT_BLOCK_HEIGHT, CREATED_AT_CORE_BLOCK_HEIGHT, UPDATED_AT,
            UPDATED_AT_BLOCK_HEIGHT, UPDATED_AT_CORE_BLOCK_HEIGHT,
        },
        Document, DocumentV0, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION,
    },
    fee::Credits,
    identifier::Identifier,
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, KeyType, Purpose,
        SecurityLevel,
    },
    platform_value::{btreemap_extensions::BTreeValueMapHelper, string_encoding::Encoding, Value},
    prelude::{DataContract, Identity, IdentityPublicKey},
    version::PlatformVersion,
    voting::{
        contender_structs::ContenderWithSerializedDocument,
        vote_choices::resource_vote_choice::ResourceVoteChoice,
        vote_polls::{
            contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll, VotePoll,
        },
        votes::{resource_vote::ResourceVote, Vote},
    },
};
use drive::query::{
    vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    },
    vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery,
    VotePollsByEndDateDriveQuery,
};
use drive_proof_verifier::types::ContestedResource;

use futures::{stream::FuturesUnordered, Future, StreamExt};
use rand::{prelude::StdRng, Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;

use super::{state::IdentityPrivateKeysMap, AppStateUpdate, CompletedTaskPayload};
use crate::{
    backend::{error::Error, AppState, BackendEvent, Task},
    config::Config,
};

#[derive(Debug, Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
    BroadcastRandomDocuments {
        data_contract_name: String,
        document_type_name: String,
        count: u16,
    },
    QueryContestedResources(DataContract, DocumentType),
    QueryDocumentsAndContestedResources {
        document_query: DocumentQuery,
        data_contract: DataContract,
        document_type: DocumentType,
    },
    QueryVoteContenders(String, Vec<Value>, String, Identifier),
    VoteOnContestedResource(VotePoll, ResourceVoteChoice),
    BroadcastDocument {
        data_contract_name: String,
        document_type_name: String,
        properties: HashMap<String, String>,
    },
    PurchaseDocument {
        data_contract: DataContract,
        document_type: DocumentType,
        document: Document,
    },
    SetDocumentPrice {
        amount: u64,
        data_contract: DataContract,
        document_type: DocumentType,
        document: Document,
    },
    TransferDocument {
        recipient_address: String,
        data_contract: DataContract,
        document_type: DocumentType,
        document: Document,
    },
}

impl AppState {
    pub(super) async fn run_document_task<'s>(
        &'s self,
        sdk: &Sdk,
        task: DocumentTask,
    ) -> BackendEvent<'s> {
        match &task {
            DocumentTask::QueryDocuments(document_query) => {
                let execution_result = Document::fetch_many(&sdk, document_query.clone())
                    .await
                    .map(CompletedTaskPayload::Documents)
                    .map_err(|e| e.to_string());
                BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result,
                }
            }
            DocumentTask::BroadcastRandomDocuments {
                data_contract_name,
                document_type_name,
                count,
            } => {
                let broadcast_stats = {
                    let data_contracts_lock = self.known_contracts.lock().await;
                    let Some(data_contract) = data_contracts_lock.get(data_contract_name) else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("Data contract not found".to_owned()),
                        };
                    };
                    let loaded_identity_lock = self.loaded_identity.lock().await;
                    let Some(identity) = loaded_identity_lock.as_ref() else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("No loaded identity".to_owned()),
                        };
                    };
                    let identity_private_keys_lock =
                        self.known_identities_private_keys.lock().await;
                    let Ok(document_type) =
                        data_contract.document_type_cloned_for_name(&document_type_name)
                    else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("Document type not found".to_owned()),
                        };
                    };

                    broadcast_random_documents(
                        sdk,
                        identity,
                        &identity_private_keys_lock,
                        data_contract,
                        &document_type,
                        *count,
                    )
                    .await
                };

                match broadcast_stats {
                    Ok(stats) => match self.refresh_loaded_identity(sdk).await {
                        Ok(updated_identity) => BackendEvent::TaskCompletedStateChange {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                stats.info_display(),
                            )),
                            app_state_update: AppStateUpdate::LoadedIdentity(updated_identity),
                        },
                        Err(_) => BackendEvent::TaskCompletedStateChange {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                stats.info_display(),
                            )),
                            app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                        },
                    },
                    Err(e) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!(
                                "Unable to start broadcasting: {}",
                                e.to_string()
                            )),
                        }
                    }
                }
            }
            DocumentTask::BroadcastDocument {
                data_contract_name,
                document_type_name,
                properties: properties_strings,
            } => {
                // Get the data contract
                let known_contracts_lock = self.known_contracts.lock().await;
                let data_contract = match known_contracts_lock.get(data_contract_name) {
                    Some(contract) => contract,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(
                                "Data contract not found in TUI known contracts".to_string()
                            ),
                        }
                    }
                };
                let data_contract_arc = Arc::new(data_contract.clone());

                // Get the document type
                let document_type = match data_contract.document_types().get(document_type_name) {
                    Some(doc_type) => doc_type,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(
                                "Document type name not found in data contract".to_string()
                            ),
                        }
                    }
                };

                // Get the identity public key
                let loaded_identity_lock = self.loaded_identity.lock().await;
                let loaded_identity = match loaded_identity_lock.as_ref() {
                    Some(identity) => identity,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("No identity loaded".to_string()),
                        }
                    }
                };
                let identity_public_key = match loaded_identity.get_first_public_key_matching(
                    Purpose::AUTHENTICATION,
                    HashSet::from([document_type.security_level_requirement()]),
                    HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                    false
                ) {
                    Some(key) => key,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err("Loaded identity does not have a public key matching the criteria for document broadcasts".to_string()),
                        }
                    }
                };

                // Get signer from loaded identity
                let mut signer = SimpleSigner::default();
                let Identity::V0(identity_v0) = loaded_identity;
                let identity_private_keys_lock = self.known_identities_private_keys.lock().await;
                for (key_id, public_key) in &identity_v0.public_keys {
                    let identity_key_tuple = (identity_v0.id, *key_id);
                    if let Some(private_key_bytes) =
                        identity_private_keys_lock.get(&identity_key_tuple)
                    {
                        signer
                            .private_keys
                            .insert(public_key.clone(), private_key_bytes.clone());
                    }
                }
                drop(identity_private_keys_lock);

                // Get the state transition entropy
                let mut rng = StdRng::from_entropy();
                let document_state_transition_entropy: [u8; 32] = rng.gen();

                // Now gather all the parameters to create a document
                // For this I am copying the logic from DocumentTypeV0::random_document_with_params()

                let id = Document::generate_document_id_v0(
                    &data_contract.id(),
                    &loaded_identity.id(),
                    &document_type_name.as_str(),
                    document_state_transition_entropy.as_slice(),
                );

                let DocumentType::V0(document_type_v0) = document_type;
                let revision = if document_type_v0.requires_revision() {
                    Some(INITIAL_REVISION)
                } else {
                    None
                };

                let created_at = if document_type_v0.required_fields().contains(CREATED_AT) {
                    let now = SystemTime::now();
                    let duration_since_epoch =
                        now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                    let milliseconds = duration_since_epoch.as_millis() as u64;
                    Some(milliseconds)
                } else {
                    None
                };

                let updated_at = if document_type_v0.required_fields().contains(UPDATED_AT) {
                    let now = SystemTime::now();
                    let duration_since_epoch =
                        now.duration_since(UNIX_EPOCH).expect("Time went backwards");
                    let milliseconds = duration_since_epoch.as_millis() as u64;
                    Some(milliseconds)
                } else {
                    None
                };

                let created_at_block_height = if document_type_v0
                    .required_fields()
                    .contains(CREATED_AT_BLOCK_HEIGHT)
                {
                    Some(0)
                } else {
                    None
                };

                let updated_at_block_height = if document_type_v0
                    .required_fields()
                    .contains(UPDATED_AT_BLOCK_HEIGHT)
                {
                    Some(0)
                } else {
                    None
                };

                let created_at_core_block_height = if document_type_v0
                    .required_fields()
                    .contains(CREATED_AT_CORE_BLOCK_HEIGHT)
                {
                    Some(0)
                } else {
                    None
                };

                let updated_at_core_block_height = if document_type_v0
                    .required_fields()
                    .contains(UPDATED_AT_CORE_BLOCK_HEIGHT)
                {
                    Some(0)
                } else {
                    None
                };

                let mut properties: BTreeMap<String, Value> = BTreeMap::new();
                let _ = properties_strings.iter().map(|(k, v)| {
                    // If the value is meant to be a number, try to convert it and insert, otherwise insert as a string
                    if let Ok(number) = Value::from(v).to_integer_broad_conversion() {
                        properties.insert(k.clone(), number);
                    } else {
                        let value = Value::Text(v.to_string());
                        properties.insert(k.clone(), value);
                    }
                });

                let document: Document = Document::V0(DocumentV0 {
                    id,
                    properties: properties.clone(),
                    owner_id: loaded_identity.id(),
                    revision,
                    created_at,
                    updated_at,
                    transferred_at: None,
                    created_at_block_height,
                    updated_at_block_height,
                    transferred_at_block_height: None,
                    created_at_core_block_height,
                    updated_at_core_block_height,
                    transferred_at_core_block_height: None,
                });

                tracing::info!("Document: {:?}", document);

                match document
                    .put_to_platform_and_wait_for_response(
                        sdk,
                        document_type.clone(),
                        document_state_transition_entropy,
                        identity_public_key.clone(),
                        data_contract_arc,
                        &signer,
                    )
                    .await
                {
                    Ok(document) => BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Ok(format!(
                            "Successfully broadcasted document with id {}",
                            document.id().to_string(Encoding::Base58)
                        )
                        .into()),
                    },
                    Err(e) => BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(format!("Failed to broadcast document: {}", e).into()),
                    },
                };
                BackendEvent::None
            }
            DocumentTask::PurchaseDocument {
                data_contract,
                document_type,
                document,
            } => {
                if let Some(loaded_identity) = self.loaded_identity.lock().await.as_ref() {
                    if let Some(public_key) = loaded_identity.get_first_public_key_matching(
                        Purpose::AUTHENTICATION,
                        HashSet::from([SecurityLevel::CRITICAL]),
                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                        false,
                    ) {
                        let price = match document
                            .properties()
                            .get_optional_integer::<Credits>("$price")
                        {
                            Ok(price) => {
                                if let Some(price) = price {
                                    price
                                } else {
                                    // no price set
                                    tracing::error!("Document is not for sale");
                                    return BackendEvent::TaskCompleted {
                                        task: Task::Document(task),
                                        execution_result: Err(format!("Document is not for sale")),
                                    };
                                }
                            }
                            Err(e) => {
                                // no optional price field found
                                return BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Err(format!(
                                        "No price field found in the document: {e}",
                                    )),
                                };
                            }
                        };

                        // Get signer from loaded_identity
                        let identity_private_keys_lock =
                            self.known_identities_private_keys.lock().await;
                        let mut signer = SimpleSigner::default();
                        let Identity::V0(identity_v0) = loaded_identity;
                        for (key_id, public_key) in &identity_v0.public_keys {
                            let identity_key_tuple = (identity_v0.id, *key_id);
                            if let Some(private_key_bytes) =
                                identity_private_keys_lock.get(&identity_key_tuple)
                            {
                                signer
                                    .private_keys
                                    .insert(public_key.clone(), private_key_bytes.clone());
                            }
                        }
                        drop(identity_private_keys_lock);

                        let data_contract_arc = Arc::new(data_contract.clone());

                        let mut new_document = document.clone();
                        new_document.bump_revision();

                        match new_document
                            .purchase_document_and_wait_for_response(
                                price,
                                sdk,
                                document_type.clone(),
                                loaded_identity.id(),
                                public_key.clone(),
                                data_contract_arc,
                                &signer,
                            )
                            .await
                        {
                            Ok(document) => match self.refresh_loaded_identity(sdk).await {
                                Ok(updated_identity) => BackendEvent::TaskCompletedStateChange {
                                    task: Task::Document(task),
                                    execution_result: Ok(format!(
                                        "Successfully purchased document with id {}",
                                        document.id().to_string(
                                            dpp::platform_value::string_encoding::Encoding::Base58
                                        )
                                    )
                                    .into()),
                                    app_state_update: AppStateUpdate::LoadedIdentity(
                                        updated_identity,
                                    ),
                                },
                                Err(_) => BackendEvent::TaskCompletedStateChange {
                                    task: Task::Document(task),
                                    execution_result: Ok(format!(
                                        "Successfully purchased document with id {} but failed to refresh identity balance after",
                                        document.id().to_string(
                                            dpp::platform_value::string_encoding::Encoding::Base58
                                        )
                                    )
                                    .into()),
                                    app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                                }
                            },
                            Err(e) => {
                                tracing::error!(
                                    "Error from purchase_document_and_wait_for_response: {}",
                                    e.to_string()
                                );
                                BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Err(format!(
                                        "Error from purchase_document_and_wait_for_response: {}",
                                        e.to_string()
                                    )),
                                }
                            }
                        }
                    } else {
                        // no matching key
                        BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!(
                                "No key suitable for purchasing documents in the loaded identity"
                            )),
                        }
                    }
                } else {
                    // no loaded identity
                    BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(format!(
                            "No loaded identity for purchasing documents"
                        )),
                    }
                }
            }
            DocumentTask::SetDocumentPrice {
                amount,
                data_contract,
                document_type,
                document,
            } => {
                if let Some(loaded_identity) = self.loaded_identity.lock().await.as_ref() {
                    if let Some(public_key) = loaded_identity.get_first_public_key_matching(
                        Purpose::AUTHENTICATION,
                        HashSet::from([SecurityLevel::CRITICAL]),
                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                        false,
                    ) {
                        // Get signer from loaded_identity
                        let identity_private_keys_lock =
                            self.known_identities_private_keys.lock().await;
                        let mut signer = SimpleSigner::default();
                        let Identity::V0(identity_v0) = loaded_identity;
                        for (key_id, public_key) in &identity_v0.public_keys {
                            let identity_key_tuple = (identity_v0.id, *key_id);
                            if let Some(private_key_bytes) =
                                identity_private_keys_lock.get(&identity_key_tuple)
                            {
                                signer
                                    .private_keys
                                    .insert(public_key.clone(), private_key_bytes.clone());
                            }
                        }
                        drop(identity_private_keys_lock);

                        let data_contract_arc = Arc::new(data_contract.clone());

                        let mut new_document = document.clone();
                        new_document.bump_revision();

                        match new_document
                            .update_price_of_document_and_wait_for_response(
                                *amount,
                                sdk,
                                document_type.clone(),
                                public_key.clone(),
                                data_contract_arc,
                                &signer,
                            )
                            .await
                        {
                            Ok(document) => match self.refresh_loaded_identity(sdk).await {
                                Ok(updated_identity) => BackendEvent::TaskCompletedStateChange {
                                    task: Task::Document(task),
                                    execution_result: Ok(format!(
                                        "Successfully updated price of document with id {}",
                                        document.id().to_string(
                                            dpp::platform_value::string_encoding::Encoding::Base58
                                        )
                                    )
                                    .into()),
                                    app_state_update: AppStateUpdate::LoadedIdentity(
                                        updated_identity,
                                    ),
                                },
                                Err(_) => BackendEvent::TaskCompletedStateChange {
                                    task: Task::Document(task),
                                    execution_result: Ok(format!(
                                        "Successfully updated price of document with id {} but failed to refresh identity balance after",
                                        document.id().to_string(
                                            dpp::platform_value::string_encoding::Encoding::Base58
                                        )
                                    )
                                    .into()),
                                    app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                                }
                            },
                            Err(e) => {
                                tracing::error!(
                                    "Error from update_price_of_document_and_wait_for_response: {}",
                                    e.to_string()
                                );
                                BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Err(format!(
                                        "Error from update_price_of_document_and_wait_for_response: {}",
                                        e.to_string()
                                    )),
                                }
                            }
                        }
                    } else {
                        // no matching key
                        BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!(
                                "No key suitable for updating document prices in the loaded identity"
                            )),
                        }
                    }
                } else {
                    // no loaded identity
                    BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(format!(
                            "No loaded identity for updating document prices"
                        )),
                    }
                }
            }
            DocumentTask::TransferDocument {
                recipient_address,
                data_contract,
                document_type,
                document,
            } => {
                let recipient_id =
                    match Identifier::from_string(&recipient_address, Encoding::Base58) {
                        Ok(id) => id,
                        Err(_) => {
                            return BackendEvent::TaskCompleted {
                                task: Task::Document(task),
                                execution_result: Ok(CompletedTaskPayload::String(
                                    "Can't parse identifier as base58 string".to_string(),
                                )),
                            }
                        }
                    };
                let loaded_identity = self.loaded_identity.lock().await;
                if let Some(identity) = loaded_identity.as_ref() {
                    let identity_public_key = match identity.get_first_public_key_matching(
                        Purpose::AUTHENTICATION,
                        HashSet::from([SecurityLevel::CRITICAL]),
                        HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
                        false,
                    ) {
                        Some(key) => key,
                        None => {
                            return BackendEvent::TaskCompleted {
                                task: Task::Document(task),
                                execution_result: Err(format!(
                                    "Error: identity doesn't have a key for document transfers"
                                )),
                            }
                        }
                    };

                    let loaded_identity_private_keys =
                        self.known_identities_private_keys.lock().await;
                    let Some(private_key) = loaded_identity_private_keys
                        .get(&(identity.id(), identity_public_key.id()))
                    else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::String(
                                "Error: corresponding private key not found for document transfer transition".to_string(),
                            )),
                        };
                    };
                    let mut signer = SimpleSigner::default();
                    signer.add_key(identity_public_key.clone(), private_key.to_vec());

                    let data_contract_arc = Arc::new(data_contract.clone());

                    let mut new_document = document.clone();
                    new_document.bump_revision();

                    match new_document
                        .transfer_document_to_identity_and_wait_for_response(
                            recipient_id,
                            sdk,
                            document_type.clone(),
                            identity_public_key.clone(),
                            data_contract_arc,
                            &signer,
                        )
                        .await
                    {
                        Ok(document) => match self.refresh_loaded_identity(sdk).await {
                            Ok(updated_identity) => BackendEvent::TaskCompletedStateChange {
                                task: Task::Document(task),
                                execution_result: Ok(format!(
                                    "Successfully transferred document with id {}",
                                    document.id().to_string(
                                        dpp::platform_value::string_encoding::Encoding::Base58
                                    )
                                )
                                .into()),
                                app_state_update: AppStateUpdate::LoadedIdentity(
                                    updated_identity,
                                ),
                            },
                            Err(_) => BackendEvent::TaskCompletedStateChange {
                                task: Task::Document(task),
                                execution_result: Ok(format!(
                                    "Successfully transferred document with id {} but failed to refresh identity balance after",
                                    document.id().to_string(
                                        dpp::platform_value::string_encoding::Encoding::Base58
                                    )
                                )
                                .into()),
                                app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                            }
                        },
                        Err(e) => BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(CompletedTaskPayload::String(
                                format!("Error during transfer_document_to_identity_and_wait_for_response: {}", e.to_string()),
                            ).to_string()),
                        },
                    }
                } else {
                    BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Ok(CompletedTaskPayload::String(
                            "No loaded identity for document transfer".to_string(),
                        )),
                    }
                }
            }
            DocumentTask::QueryDocumentsAndContestedResources {
                document_query,
                data_contract,
                document_type,
            } => {
                // Fetch documents
                let documents_result = Document::fetch_many(&sdk, document_query.clone()).await;

                // Fetch contested resources
                let contested_resources_result =
                    if let Some(contested_index) = document_type.find_contested_index() {
                        let query = VotePollsByDocumentTypeQuery {
                            contract_id: data_contract.id(),
                            document_type_name: document_type.name().to_string(),
                            index_name: contested_index.name.clone(),
                            start_at_value: None,
                            start_index_values: vec!["dash".into()], // hardcoded for dpns
                            end_index_values: vec![],
                            limit: None,
                            order_ascending: true,
                        };

                        ContestedResource::fetch_many(sdk, query).await
                    } else {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(
                                "No contested index for this document type".to_owned()
                            ),
                        };
                    };

                // Combine results
                match (documents_result, contested_resources_result) {
                    (Ok(documents), Ok(contested_resources)) => BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Ok(CompletedTaskPayload::DocumentsAndContestedResources(
                            documents,
                            contested_resources,
                        )),
                    },
                    (Err(documents_err), _) => BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(format!(
                            "Failed to fetch documents: {}",
                            documents_err
                        )),
                    },
                    (_, Err(contested_resources_err)) => BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(format!(
                            "Failed to fetch contested resources: {}",
                            contested_resources_err
                        )),
                    },
                }
            }
            DocumentTask::QueryVoteContenders(
                index_name,
                index_values,
                document_type_name,
                contract_id,
            ) => {
                let vote_poll = ContestedDocumentResourceVotePoll {
                    index_name: index_name.to_string(),
                    index_values: index_values.to_vec(),
                    document_type_name: document_type_name.to_string(),
                    contract_id: *contract_id,
                };

                let contenders_query = ContestedDocumentVotePollDriveQuery {
                    limit: None,
                    offset: None,
                    start_at: None,
                    vote_poll: vote_poll.clone(),
                    allow_include_locked_and_abstaining_vote_tally: true,
                    result_type:
                        ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                };

                let contenders = match ContenderWithSerializedDocument::fetch_many(
                    sdk,
                    contenders_query.clone(),
                )
                .await
                {
                    Ok(contenders) => contenders,
                    Err(e) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!("{e}")),
                        }
                    }
                };

                let now: DateTime<Utc> = Utc::now();
                let start_time_dt = now - Duration::weeks(2);
                let end_time_dt = now + Duration::weeks(2);
                let start_time = Some((start_time_dt.timestamp_millis() as u64, true));
                let end_time = Some((end_time_dt.timestamp_millis() as u64, true));

                let end_time_query = VotePollsByEndDateDriveQuery {
                    start_time,
                    end_time,
                    limit: None,
                    offset: None,
                    order_ascending: true,
                };

                let vote_polls_by_timestamp = match VotePoll::fetch_many(sdk, end_time_query).await
                {
                    Ok(polls_by_timestamp) => polls_by_timestamp,
                    Err(error) => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!(
                                "Failed to fetch vote polls by timestamp: {error}"
                            )),
                        }
                    }
                };

                let mut end_timestamp = None;
                for (timestamp, vote_polls) in &vote_polls_by_timestamp.0 {
                    if vote_polls.iter().any(|vp| match vp {
                        VotePoll::ContestedDocumentResourceVotePoll(inner_vote_poll) => {
                            inner_vote_poll == &vote_poll
                        }
                    }) {
                        end_timestamp = Some(*timestamp);
                        break;
                    }
                }

                BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result: Ok(CompletedTaskPayload::ContestedResourceContenders(
                        contenders_query.vote_poll,
                        contenders,
                        end_timestamp,
                    )),
                }
            }
            DocumentTask::QueryContestedResources(data_contract, document_type) => {
                if let Some(contested_index) = document_type.find_contested_index() {
                    let query = VotePollsByDocumentTypeQuery {
                        contract_id: data_contract.id(),
                        document_type_name: document_type.name().to_string(),
                        index_name: contested_index.name.clone(),
                        start_at_value: None,
                        start_index_values: vec!["dash".into()], // hardcoded for dpns
                        end_index_values: vec![],
                        limit: None,
                        order_ascending: true,
                    };

                    let contested_resources = ContestedResource::fetch_many(sdk, query).await;

                    match contested_resources {
                        Ok(resources) => BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Ok(CompletedTaskPayload::ContestedResources(
                                resources,
                            )),
                        },
                        Err(e) => BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(format!("{e}")),
                        },
                    }
                } else {
                    BackendEvent::TaskCompleted {
                        task: Task::Document(task),
                        execution_result: Err(
                            "No contested index for this document type".to_owned()
                        ),
                    }
                }
            }
            DocumentTask::VoteOnContestedResource(vote_poll, vote_choice) => {
                let mut vote = Vote::default();

                // Get signer from loaded_identity
                // Convert loaded_identity to SimpleSigner
                let identity_private_keys_lock = self.known_identities_private_keys.lock().await;
                let loaded_identity_lock = match self.loaded_identity.lock().await.clone() {
                    Some(identity) => identity,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(
                                "No loaded identity for signing vote transaction".to_string(),
                            ),
                        };
                    }
                };

                let mut signer = SimpleSigner::default();
                let Identity::V0(identity_v0) = &loaded_identity_lock;
                for (key_id, public_key) in &identity_v0.public_keys {
                    let identity_key_tuple = (identity_v0.id, *key_id);
                    if let Some(private_key_bytes) =
                        identity_private_keys_lock.get(&identity_key_tuple)
                    {
                        signer
                            .private_keys
                            .insert(public_key.clone(), private_key_bytes.clone());
                    }
                }

                let voting_public_key = match loaded_identity_lock.get_first_public_key_matching(
                    Purpose::VOTING,
                    HashSet::from(SecurityLevel::full_range()),
                    HashSet::from(KeyType::all_key_types()),
                    false,
                ) {
                    Some(voting_key) => voting_key,
                    None => {
                        return BackendEvent::TaskCompleted {
                            task: Task::Document(task),
                            execution_result: Err(
                                "No voting key in the loaded identity. Are you sure it's a masternode identity?".to_string()
                            ),
                        };
                    }
                };

                match vote {
                    Vote::ResourceVote(ref mut resource_vote) => match resource_vote {
                        ResourceVote::V0(ref mut resource_vote_v0) => {
                            resource_vote_v0.vote_poll = vote_poll.clone();
                            resource_vote_v0.resource_vote_choice = *vote_choice;
                            let pro_tx_hash = self
                                .loaded_identity_pro_tx_hash
                                .lock()
                                .await
                                .expect("Expected a proTxHash in AppState");
                            match vote
                                .put_to_platform_and_wait_for_response(
                                    pro_tx_hash,
                                    voting_public_key,
                                    sdk,
                                    &signer,
                                    None,
                                )
                                .await
                            {
                                Ok(_) => BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Ok(CompletedTaskPayload::String(
                                        "Vote cast successfully".to_string(),
                                    )),
                                },
                                Err(e) => BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Err(e.to_string()),
                                },
                            }
                        }
                    },
                }
            }
        }
    }
}

struct BroadcastRandomDocumentsStats {
    total: u16,
    completed: u16,
    last_error: Option<String>,
}

impl BroadcastRandomDocumentsStats {
    fn info_display(&self) -> String {
        format!(
            "Broadcast random documents results:
Completed {} of {}
Last error: {}",
            self.completed,
            self.total,
            self.last_error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or("".to_owned())
        )
    }
}

async fn broadcast_random_documents<'s>(
    sdk: &Sdk,
    identity: &Identity,
    identity_private_keys: &IdentityPrivateKeysMap,
    data_contract: &DataContract,
    document_type: &DocumentType,
    count: u16,
) -> Result<BroadcastRandomDocumentsStats, Error> {
    let mut std_rng = StdRng::from_entropy();

    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([document_type.security_level_requirement()]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
            false,
        )
        .ok_or(Error::DocumentSigningError(
            "No public key matching security level requirements".to_string(),
        ))?;

    let Some(private_key) = identity_private_keys.get(&(identity.id(), identity_public_key.id()))
    else {
        // TODO inappropriate error type
        return Err(Error::IdentityTopUpError(format!(
            "expected private keys, but we only have private keys for {:?}, trying to get {:?} : \
             {}",
            identity_private_keys
                .keys()
                .map(|(id, key_id)| (id, key_id))
                .collect::<BTreeMap<_, _>>(),
            identity.id(),
            identity_public_key.id(),
        )));
    };

    let data_contract = Arc::new(data_contract.clone());
    let mut signer = SimpleSigner::default();
    signer.add_key(identity_public_key.clone(), private_key.to_vec());

    fn put_random_document<'a, 'r>(
        sdk: &'a Sdk,
        document_type: &'a DocumentType,
        identity: &'a Identity,
        rng: &'r mut StdRng,
        signer: &'a SimpleSigner,
        identity_public_key: &'a IdentityPublicKey,
        data_contract: Arc<DataContract>,
    ) -> impl Future<Output = Result<(), String>> + 'a {
        let document_state_transition_entropy: [u8; 32] = rng.gen();
        let time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock may have gone backwards")
            .as_millis();

        let random_document = document_type
            .random_document_with_params(
                identity.id(),
                document_state_transition_entropy.into(),
                Some(time_ms as u64),
                None,
                None,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                rng,
                sdk.version(),
            )
            .expect("expected a random document");

        async move {
            random_document
                .put_to_platform_and_wait_for_response(
                    sdk,
                    document_type.clone(),
                    document_state_transition_entropy,
                    identity_public_key.clone(),
                    data_contract,
                    signer,
                )
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }

    let mut futures: FuturesUnordered<_> = iter::repeat_with(|| {
        put_random_document(
            sdk,
            document_type,
            identity,
            &mut std_rng,
            &signer,
            identity_public_key,
            Arc::clone(&data_contract),
        )
    })
    .take(count as usize)
    .collect();

    let mut completed = 0;
    let mut last_error = None;

    while let Some(result) = futures.next().await {
        match result {
            Ok(_) => completed += 1,
            Err(e) => last_error = Some(e),
        }
    }

    Ok(BroadcastRandomDocumentsStats {
        total: count,
        completed,
        last_error,
    })
}
