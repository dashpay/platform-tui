//! Contract backend logic.
use dpp::{
    prelude::{DataContract, Identifier},
    serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure,
    version::PlatformVersion,
};
use tuirealm::props::{PropValue, TextSpan};

use crate::app::error::Error;

pub(super) fn data_contract_bytes_to_spans(bytes: &[u8]) -> Result<Vec<PropValue>, Error> {
    let data_contract =
        DataContract::versioned_deserialize(&bytes, false, PlatformVersion::latest())?;
    let textual = toml::to_string_pretty(&data_contract).expect("data contract is serializable");
    Ok(textual
        .lines()
        .map(|line| PropValue::TextSpan(TextSpan::new(line)))
        .collect())
}
