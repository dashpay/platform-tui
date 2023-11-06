//! Backend functionality related to identities.

use dapi_grpc::{
    core::v0::{
        self as core_proto, transactions_with_proofs_request::FromBlock,
        transactions_with_proofs_response, InstantSendLockMessages, TransactionsWithProofsResponse,
    },
    platform::v0::{
        self as platform_proto, get_identity_response::Result as ProtoResult, GetIdentityResponse,
    },
};
use dpp::{
    platform_value::string_encoding::Encoding,
    prelude::{Identifier, Identity},
    serialization::PlatformDeserializable,
};
use rs_dapi_client::{DapiClient, DapiRequest, RequestSettings};

#[derive(Debug, thiserror::Error)]
pub(crate) enum IdentityFetchError {
    #[error("error serializing identity id from b58 string")]
    IdentifierSerialization,
    #[error("error deserializing identity from proto bytes")]
    IdentityDeserialization,
    #[error("DAPI transport error")]
    DapiError,
}

pub(crate) async fn fetch_identity_by_b58_id(
    client: &mut DapiClient,
    b58_id: String,
) -> Result<Identity, IdentityFetchError> {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let identifier = Identifier::from_string(b58_id.as_str(), Encoding::Base58)
        .map_err(|_| IdentityFetchError::IdentifierSerialization)?;
    let request = platform_proto::GetIdentityRequest {
        id: identifier.to_vec(),
        prove: false,
    };
    // let response = request.execute(client, RequestSettings::default()).await;
    // if let Ok(GetIdentityResponse {
    //     result: Some(ProtoResult::Identity(bytes)),
    //     ..
    // }) = response
    // {
    //     Ok(Identity::deserialize_from_bytes(&bytes)
    //         .map_err(|_| IdentityFetchError::IdentityDeserialization)?)
    // } else {
    //     Err(IdentityFetchError::DapiError)
    // }

    Ok(Identity::random_identity(2, None, &dpp::version::PlatformVersion::latest()).unwrap())
}
