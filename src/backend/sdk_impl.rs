//! Implementation of dash sdk traits for Backend
//!
//! This file contains implementation of [ContextProvider] and
//! [Wallet](dash_platform_sdk::wallet::Wallet) trait for [Backend] struct.
use std::sync::Arc;

use dapi_grpc::tonic::async_trait;
use dash_platform_sdk::{
    error::ContextProviderError,
    platform::ContextProvider,
    wallet::{ListUnspentResultEntry, Wallet as SdkWallet},
};
use dpp::{
    bls_signatures::PrivateKey,
    data_contract::DataContract,
    identity::{
        identity_public_key::v0::IdentityPublicKeyV0, signer::Signer,
        state_transition::asset_lock_proof::AssetLockProof, IdentityPublicKey, KeyID, KeyType,
    },
    platform_value::{BinaryData, Identifier},
    ProtocolError,
};

use super::Backend;

impl ContextProvider for Backend {
    fn get_data_contract(
        &self,
        id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let id_base58 = id.to_string(dpp::platform_value::string_encoding::Encoding::Base58);

        // double-check that we have Tokio runtime available
        let _handle = tokio::runtime::Handle::try_current().map_err(|e| {
            ContextProviderError::Config(format!("no tokio runtime detected: {}", e))
        })?;

        let lock = tokio::task::block_in_place(|| self.state().known_contracts.blocking_lock());

        Ok(lock.get(&id_base58).map(|c| Arc::new(c.clone())))
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32], // quorum hash is 32 bytes
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // TODO: this is just a temporary "hack" to get quorum keys from the SDK, using
        // Dash Core API. This should be replaced with a proper quorum provider using
        // SPV.
        self.core
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }
}
#[async_trait]

impl SdkWallet for Backend {
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, dash_platform_sdk::Error> {
        todo!("not implemented yet")
    }

    async fn identity_public_key(
        &self,
        purpose: &dpp::identity::Purpose,
    ) -> Option<dpp::prelude::IdentityPublicKey> {
        let wallet = tokio::task::block_in_place(|| self.state().loaded_wallet.blocking_lock());
        let key = wallet.as_ref()?.public_key();
        // TODO: Implement correctly
        Some(IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0 as KeyID,
            purpose: dpp::identity::Purpose::AUTHENTICATION,
            security_level: dpp::identity::SecurityLevel::lowest_level(),
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData(key.to_bytes().to_vec()),
            disabled_at: None,
        }))
    }

    async fn lock_assets(
        &self,
        amount: u64,
    ) -> Result<(AssetLockProof, PrivateKey), dash_platform_sdk::Error> {
        todo!("not implemented yet")
    }

    /// Return balance of the wallet, in satoshis.
    async fn core_balance(&self) -> Result<u64, dash_platform_sdk::Error> {
        todo!("not implemented yet")
    }

    /// Return list of unspent transactions with summarized balance at least
    /// `sum`
    async fn core_utxos(
        &self,
        sum: Option<u64>,
    ) -> Result<Vec<ListUnspentResultEntry>, dash_platform_sdk::Error> {
        todo!("not implemented yet")
    }
}
impl Signer for Backend {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let wallet = tokio::task::block_in_place(|| self.state().loaded_wallet.blocking_lock())
            .as_ref()
            .ok_or(ProtocolError::Generic("no active wallet found".to_string()))?;

        todo!("not implemented yet")
    }
}
