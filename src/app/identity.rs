//! Identities backend logic.

use dapi_grpc::platform::v0::{
    self as platform_proto, get_identity_response::Result as ProtoResult, GetIdentityResponse,
};
use dpp::{prelude::Identity, serialization::PlatformDeserializable, ProtocolError};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;
use rs_dapi_client::{DapiClient, DapiRequest, RequestSettings};
use tuirealm::props::{PropValue, TextSpan};
use crate::app::error::Error;
use crate::app::state::AppState;

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
    let identifier = Identifier::from_string(b58_id.as_str(), Encoding::Base58)?;
    let request = platform_proto::GetIdentityRequest { id: identifier.to_vec(), prove: false };
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

impl AppState {
    pub fn register_identity(&mut self) {
        let Some(wallet) = self.loaded_wallet.as_ref() else {
            return;
        };

        //// Core steps

        // first we create the wallet registration transaction, this locks funds that we can transfer from core to platform
        let (transaction, private_key) = wallet.registration_transaction();

        self.identity_creation_private_key = Some(private_key.inner.secret_bytes());

        // create the bloom filter

        // let bloom_filter = wallet.bloom_filter() todo() -> Sam

        // we should subscribe and listen to transactions from core todo() -> Evgeny

        // we need to broadcast the transaction to core todo() -> Evgeny

        // Get the instant send lock back todo() -> Evgeny

        //// Platform steps

        // Create the identity create state transition todo() -> Sam

        // Subscribe to state transition result todo() -> Evgeny

        // Through sdk send this transaction and get back proof that the identity was created todo() -> Evgeny

        // Verify proof and get identity todo() -> Sam

        // Add Identity as the current identity in the state todo() -> Sam
    }
}