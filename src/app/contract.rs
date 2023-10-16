//! Contract backend logic.

use dapi_grpc::platform::v0::{self as platform_proto, get_data_contract_response::Result as ProtoResult, GetDataContractResponse};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::{DataContract, Identifier};
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use rs_dapi_client::{DapiClient, DapiRequest, RequestSettings};
use tuirealm::props::{PropValue, TextSpan};
use crate::app::error::Error;

pub(super) fn data_contract_bytes_to_spans(bytes: &[u8]) -> Result<Vec<PropValue>, Error> {
    let data_contract = DataContract::versioned_deserialize(&bytes, false, PlatformVersion::latest())?;
    let textual = toml::to_string_pretty(&data_contract).expect("data contract is serializable");
    Ok(textual
        .lines()
        .map(|line| PropValue::TextSpan(TextSpan::new(line)))
        .collect())
}

pub(super) fn fetch_data_contract_bytes_by_b58_id(
    client: &mut DapiClient,
    b58_id: String,
) -> Result<Vec<u8>, Error> {
    let identifier = Identifier::from_string(b58_id.as_str(), Encoding::Base58)?;
    let request = platform_proto::GetDataContractRequest { id: identifier.to_vec(), prove: false };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let response = runtime.block_on(request.execute(client, RequestSettings::default()));
    if let Ok(GetDataContractResponse {
                  result: Some(ProtoResult::DataContract(bytes)),
                  ..
              }) = response
    {
        Ok(bytes)
    } else {
        Err(Error::DapiError)
    }
}
