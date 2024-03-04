//! Random documents broadcasting functionality and tools.

use std::{
    collections::{BTreeMap, HashSet},
    iter,
    sync::Arc,
};

use dapi_grpc::core::v0::{get_status_response, GetStatusRequest, GetStatusResponse};
use dpp::{
    data_contract::{
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
        DataContract,
    },
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, Identity, KeyType, Purpose,
    },
    platform_value::{Bytes32, Value},
    system_data_contracts::dashpay_contract::v1::document_types::contact_request::properties::CORE_HEIGHT_CREATED_AT,
    version::PlatformVersion,
};
use futures::{stream::FuturesUnordered, StreamExt};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
use rs_sdk::{platform::transition::put_document::PutDocument, Sdk};
use simple_signer::signer::SimpleSigner;

use crate::backend::{error::Error, state::IdentityPrivateKeysMap};

pub(super) struct BroadcastRandomDocumentsStats {
    total: u16,
    completed: u16,
    last_error: Option<String>,
}

impl BroadcastRandomDocumentsStats {
    pub fn info_display(&self) -> String {
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

pub(super) async fn broadcast_random_documents<'s>(
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

    let substitutions = prepare_substitutions(&sdk).await?;

    let mut futures: FuturesUnordered<_> = iter::repeat_with(|| {
        let entropy = Bytes32(std_rng.gen());
        let data_contract = Arc::clone(&data_contract);
        let signer = &signer;
        let substitutions = &substitutions;
        async move {
            let documents = document_type.random_documents_faker(
                identity.id(),
                &entropy,
                1,
                &PlatformVersion::latest(),
                substitutions,
            )?;
            documents[0]
                .put_to_platform_and_wait_for_response(
                    sdk,
                    document_type.clone(),
                    entropy.0,
                    identity_public_key.clone(),
                    data_contract,
                    signer,
                )
                .await
        }
    })
    .take(count as usize)
    .collect();

    let mut completed = 0;
    let mut last_error = None;

    while let Some(result) = futures.next().await {
        match result {
            Ok(_) => completed += 1,
            Err(e) => last_error = Some(e.to_string()),
        }
    }

    Ok(BroadcastRandomDocumentsStats {
        total: count,
        completed,
        last_error,
    })
}

async fn prepare_substitutions(sdk: &Sdk) -> Result<BTreeMap<&str, Value>, rs_sdk::Error> {
    let GetStatusResponse {
        chain: Some(get_status_response::Chain { blocks_count, .. }),
        ..
    } = sdk
        .execute(GetStatusRequest {}, RequestSettings::default())
        .await?
    else {
        return Err(rs_sdk::Error::Generic(
            "malformed status response".to_owned(),
        ));
    };

    let mut substitutions = BTreeMap::new();
    substitutions.insert(CORE_HEIGHT_CREATED_AT, blocks_count.into());

    Ok(substitutions)
}
