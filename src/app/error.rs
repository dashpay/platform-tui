use dapi_grpc::tonic::Status;
use dpp::ProtocolError;
use rs_dapi_client::DapiClientError;

use crate::app::error::Error::{ParsingError, SdkError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("error while parsing an identity")]
    ParsingError(#[from] ProtocolError),
    #[error("ID encoding error")]
    Base58IdEncoding(#[from] bs58::decode::Error),
    #[error("Insight error {0}")]
    InsightError(String),
    #[error("Wallet error {0}")]
    WalletError(String),
    #[error("SDK error {0} {1}")]
    SdkExplainedError(String, dash_platform_sdk::Error),
    #[error("SDK error {0}")]
    SdkError(#[from] dash_platform_sdk::Error),
    #[error("Identity registration error {0}")]
    IdentityRegistrationError(String),
}

impl From<dpp::platform_value::Error> for Error {
    fn from(value: dpp::platform_value::Error) -> Self {
        ParsingError(ProtocolError::ValueError(value))
    }
}

impl From<DapiClientError<Status>> for Error {
    fn from(value: DapiClientError<Status>) -> Self {
        SdkError(value.into())
    }
}
