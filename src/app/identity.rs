//! Identities backend logic.

use dapi_grpc::platform::v0::{
    self as platform_proto, get_identity_response::Result as ProtoResult, GetIdentityResponse,
};
use dpp::{prelude::Identity, serialization::PlatformDeserializable, ProtocolError};
use rs_dapi_client::{DapiClient, DapiRequest, RequestSettings};
use tuirealm::props::{PropValue, TextSpan};

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("DAPI transport error")]
    DapiError,
    #[error("error while parsing an identity")]
    ParsingError(#[from] ProtocolError),
    #[error("ID encoding error")]
    Base58IdEncoding(#[from] bs58::decode::Error),
}

pub(super) fn identity_bytes_to_spans(bytes: &[u8]) -> Result<Vec<PropValue>, Error> {
    let identity = Identity::deserialize_from_bytes(&bytes)?;

    let textual = toml::to_string_pretty(&identity).expect("identity is serializable");
    Ok(textual
        .lines()
        .map(|line| PropValue::TextSpan(TextSpan::new(line)))
        .collect())
}

pub(super) fn fetch_identity_bytes_by_b58_id(
    client: &mut DapiClient,
    b58_id: String,
) -> Result<Vec<u8>, Error> {
    let id = bs58::decode(b58_id).into_vec()?;
    let request = platform_proto::GetIdentityRequest { id, prove: false };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let response = runtime.block_on(request.execute(client, RequestSettings::default()));
    if let Ok(GetIdentityResponse {
        result: Some(ProtoResult::Identity(bytes)),
        ..
    }) = response
    {
        Ok(bytes)
    } else {
        Err(Error::DapiError)
    }
}
