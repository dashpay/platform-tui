use std::{
    collections::{BTreeMap, HashSet},
    iter,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use dash_sdk::{
    platform::{
        transition::{
            purchase_document::PurchaseDocument, put_document::PutDocument,
            update_price_of_document::UpdatePriceOfDocument,
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
            random_document::{CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType},
            DocumentType,
        },
    },
    document::{Document, DocumentV0Getters, DocumentV0Setters},
    fee::Credits,
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, KeyType, Purpose,
        SecurityLevel,
    },
    platform_value::btreemap_extensions::BTreeValueMapHelper,
    prelude::{DataContract, Identity, IdentityPublicKey},
};
use futures::{stream::FuturesUnordered, Future, StreamExt};
use rand::{prelude::StdRng, Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;

use super::{state::IdentityPrivateKeysMap, AppStateUpdate, CompletedTaskPayload};
use crate::backend::{error::Error, AppState, BackendEvent, Task};

#[derive(Debug, Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
    BroadcastRandomDocuments {
        data_contract_name: String,
        document_type_name: String,
        count: u16,
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
                    let identity_private_keys_lock = self.identity_private_keys.lock().await;
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
                    Ok(stats) => match self.refresh_identity(sdk).await {
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
                        let identity_private_keys_lock = self.identity_private_keys.lock().await;
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
                            Ok(document) => {
                                tracing::info!(
                                    "Successfully purchased document with id {}",
                                    document.id().to_string(
                                        dpp::platform_value::string_encoding::Encoding::Base58
                                    )
                                );
                                BackendEvent::TaskCompleted {
                                    task: Task::Document(task),
                                    execution_result: Ok(format!(
                                        "Successfully purchased document with id {}",
                                        document.id().to_string(
                                            dpp::platform_value::string_encoding::Encoding::Base58
                                        )
                                    )
                                    .into()),
                                }
                            }
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
                    ) {
                        // Get signer from loaded_identity
                        let identity_private_keys_lock = self.identity_private_keys.lock().await;
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
                            Ok(document) => BackendEvent::TaskCompleted {
                                task: Task::Document(task),
                                execution_result: Ok(format!(
                                    "Successfully updated price of document with id {}",
                                    document.id().to_string(
                                        dpp::platform_value::string_encoding::Encoding::Base58
                                    )
                                )
                                .into()),
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
