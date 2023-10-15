use dpp::ProtocolError;
use crate::app::error::Error::ParsingError;

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("DAPI transport error")]
    DapiError,
    #[error("error while parsing an identity")]
    ParsingError(#[from] ProtocolError),
    #[error("ID encoding error")]
    Base58IdEncoding(#[from] bs58::decode::Error),
}

impl From<dpp::platform_value::Error> for Error {
    fn from(value: dpp::platform_value::Error) -> Self {
        ParsingError(ProtocolError::ValueError(value))
    }
}