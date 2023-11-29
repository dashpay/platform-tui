use std::str::FromStr;

use dash_sdk::sdk::Uri;
use rs_dapi_client::AddressList;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
/// Configuration for platform explorer.
///
/// Content of this configuration is loaded from environment variables or `.env`
/// file when the [Config::load()] is called.
/// Variable names in the enviroment and `.env` file must be prefixed with
/// either [LOCAL_EXPLORER_](Config::CONFIG_PREFIX) or
/// [TESTNET_EXPLORER_](Config::CONFIG_PREFIX) and written as
/// SCREAMING_SNAKE_CASE (e.g. `EXPLORER_DAPI_ADDRESSES`).
pub struct Config {
    /// Hostname of the Dash Platform node to connect to
    pub dapi_addresses: String,
    /// Host of the Dash Core RPC interface
    pub core_host: String,
    /// Port of the Dash Core RPC interface
    pub core_port: u16,
    /// Username for Dash Core RPC interface
    pub core_user: String,
    /// Password for Dash Core RPC interface
    pub core_password: String,
    /// URL of the Insight API
    pub insight_api_url: String,
}

impl Config {
    /// Prefix of configuration options in the environment variables and `.env`
    /// file.
    const LOCAL_CONFIG_PREFIX: &'static str = "LOCAL_EXPLORER_";
    /// Prefix of configuration options in the environment variables and `.env`
    /// file.
    const TESTNET_CONFIG_PREFIX: &'static str = "TESTNET_EXPLORER_";

    /// Loads a local configuration from operating system environment variables
    /// and `.env` file.
    ///
    /// Create new [Config] with data from environment variables and
    /// `.env` file. Variable names in the
    /// environment and `.env` file must be converted to SCREAMING_SNAKE_CASE
    /// and prefixed with [LOCAL_EXPLORER_](Config::CONFIG_PREFIX).
    pub fn load_local() -> Self {
        // load config from .env file
        if let Err(err) = dotenvy::from_path(".env") {
            tracing::warn!(?err, "failed to load config file");
        }

        let config: Self = envy::prefixed(Self::LOCAL_CONFIG_PREFIX)
            .from_env()
            .expect("configuration error");

        if !config.is_valid() {
            panic!("invalid configuration: {:?}", config);
        }

        config
    }

    /// Loads a local configuration from operating system environment variables
    /// and `.env` file.
    ///
    /// Create new [Config] with data from environment variables and
    /// `.env` file. Variable names in the
    /// environment and `.env` file must be converted to SCREAMING_SNAKE_CASE
    /// and prefixed with [TESTNET_EXPLORER_](Config::CONFIG_PREFIX).
    pub fn load_testnet() -> Self {
        // load config from .env file
        if let Err(err) = dotenvy::from_path(".env") {
            tracing::warn!(?err, "failed to load config file");
        }

        let config: Self = envy::prefixed(Self::TESTNET_CONFIG_PREFIX)
            .from_env()
            .expect("configuration error");

        if !config.is_valid() {
            panic!("invalid configuration: {:?}", config);
        }

        config
    }

    /// Check if configuration is set
    pub fn is_valid(&self) -> bool {
        !self.core_user.is_empty()
            && !self.core_password.is_empty()
            && self.core_port != 0
            && !self.dapi_addresses.is_empty()
            && Uri::from_str(&self.insight_api_url).is_ok()
    }

    /// List of DAPI addresses
    pub fn dapi_address_list(&self) -> AddressList {
        AddressList::from(self.dapi_addresses.as_str())
    }

    /// Insight API URI
    pub fn insight_api_uri(&self) -> Uri {
        Uri::from_str(&self.insight_api_url).expect("invalid insight API URL")
    }
}
