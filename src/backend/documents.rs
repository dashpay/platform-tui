use std::collections::BTreeMap;
use std::sync::Arc;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use dash_platform_sdk::platform::transition::put_document::PutDocument;
use dash_platform_sdk::{
    platform::{transition::broadcast::BroadcastStateTransition, DocumentQuery, FetchMany},
    Sdk,
};
use dpp::prelude::DataContract;
use dpp::{
    data_contract::document_type::{
        accessors::DocumentTypeV0Getters,
        random_document::{CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType},
        DocumentType,
    },
    document::{Document, DocumentV0Getters},
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, KeyType, Purpose,
    },
    state_transition::{
        documents_batch_transition::{
            methods::v0::DocumentsBatchTransitionMethodsV0, DocumentsBatchTransition,
        },
        proof_result::StateTransitionProofResult,
    },
};
use rand::{prelude::StdRng, Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;

use super::CompletedTaskPayload;
use crate::backend::{error::Error, stringify_result, AppState, BackendEvent, Task};

#[derive(Clone)]
pub(crate) enum DocumentTask {
    QueryDocuments(DocumentQuery),
    BroadcastRandomDocument(DataContract, DocumentType),
}

impl AppState {
    pub(super) async fn run_document_task<'s>(
        &self,
        sdk: &mut Sdk,
        task: DocumentTask,
    ) -> BackendEvent<'s> {
        match &task {
            DocumentTask::QueryDocuments(document_query) => {
                let execution_result = Document::fetch_many(sdk, document_query.clone())
                    .await
                    .map(|docs| CompletedTaskPayload::Documents(docs))
                    .map_err(|e| e.to_string());
                BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result,
                }
            }
            DocumentTask::BroadcastRandomDocument(data_contract, document_type) => {
                let execution_result = self
                    .broadcast_random_document(sdk, data_contract, document_type)
                    .await
                    .map(|doc| CompletedTaskPayload::Document(doc))
                    .map_err(|e| e.to_string());
                BackendEvent::TaskCompleted {
                    task: Task::Document(task),
                    execution_result,
                }
            }
        }
    }

    pub(crate) async fn broadcast_random_document<'s>(
        &'s self,
        sdk: &mut Sdk,
        data_contract: &DataContract,
        document_type: &DocumentType,
    ) -> Result<Document, Error> {
        let mut std_rng = StdRng::from_entropy();

        let mut loaded_identity = self.loaded_identity.lock().await;
        let Some(identity) = loaded_identity.as_mut() else {
            return Err(Error::IdentityTopUpError("No identity loaded".to_string()));
        };

        let identity_public_key = identity
            .get_first_public_key_matching(
                Purpose::AUTHENTICATION,
                HashSet::from([document_type.security_level_requirement()]),
                HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
            )
            .ok_or(Error::DocumentSigningError(
                "No public key matching security level requirements".to_string(),
            ))?;

        let loaded_identity_private_keys = self.identity_private_keys.lock().await;
        let Some(private_key) =
            loaded_identity_private_keys.get(&(identity.id(), identity_public_key.id()))
        else {
            return Err(Error::IdentityTopUpError(format!(
                "expected private keys, but we only have private keys for {:?}, trying to get {:?} : {}",
                loaded_identity_private_keys.keys().map(|(id, key_id)| (id, key_id)).collect::<BTreeMap<_,_>>(),
                identity.id(),
                identity_public_key.id(),
            )));
        };

        let document_state_transition_entropy: [u8; 32] = std_rng.gen();

        let time_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

        let mut random_document = document_type
            .random_document_with_params(
                identity.id(),
                document_state_transition_entropy.into(),
                time_ms as u64,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                &mut std_rng,
                sdk.version(),
            )
            .expect("expected a random document");

        let mut signer = SimpleSigner::default();

        signer.add_key(identity_public_key.clone(), private_key.clone().to_bytes());

        let data_contract = data_contract.clone();
        
        let document = random_document
            .put_to_platform_and_wait_for_response(
                sdk,
                document_type.clone(),
                document_state_transition_entropy,
                identity_public_key.clone(),
                Arc::new(data_contract),
                &signer,
            )
            .await?;

        Ok(document)
    }
}
