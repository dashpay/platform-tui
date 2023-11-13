//! Backend functionality related to identities.

use dash_platform_sdk::{platform::Fetch, Sdk};
use dpp::{
    platform_value::string_encoding::Encoding,
    prelude::{Identifier, Identity},
};

use super::stringify_result;

pub(super) async fn fetch_identity_by_b58_id(
    sdk: &mut Sdk,
    base58_id: &str,
) -> Result<String, String> {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let id_bytes = Identifier::from_string(base58_id, Encoding::Base58)
        .map_err(|_| "can't parse identifier as base58 string".to_owned())?;

    let fetch_result = Identity::fetch(sdk, id_bytes).await;
    stringify_result(fetch_result)
}
