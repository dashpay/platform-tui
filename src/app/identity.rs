//! Identities backend logic.

use dapi_grpc::core::v0::{
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetTransactionRequest,
};
use std::collections::BTreeMap;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::{get_identity_balance_request, GetIdentityBalanceRequest};
use dash_platform_sdk::platform::Fetch;

use dash_platform_sdk::platform::transition::put_identity::PutIdentity;
use dash_platform_sdk::Sdk;
use dpp::dashcore::psbt::serialize::Serialize;
use dpp::dashcore::{InstantLock, OutPoint, Transaction};
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::prelude::{AssetLockProof, Identity, IdentityPublicKey};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_dapi_client::{Dapi, DapiClientError, RequestSettings};
use simple_signer::signer::SimpleSigner;
use tuirealm::props::{PropValue, TextSpan};

use crate::app::error::Error;
use crate::app::state::AppState;

pub(super) fn identity_to_spans(identity: &Identity) -> Result<Vec<PropValue>, Error> {
    let textual = toml::to_string_pretty(identity).expect("identity is serializable");
    Ok(textual
        .lines()
        .map(|line| PropValue::TextSpan(TextSpan::new(line)))
        .collect())
}


impl AppState {
    pub(crate) async fn refresh_identity_balance(
        &mut self,
        sdk: &Sdk,
    ) -> Result<(), Error> {
        if let Some(identity) = self.loaded_identity.as_mut() {
            let balance = u64::fetch(sdk, GetIdentityBalanceRequest { version: Some(get_identity_balance_request::Version::V0(GetIdentityBalanceRequestV0 { id: identity.id().to_vec(), prove: true })) }).await?;
            if let Some(balance) = balance {
                identity.set_balance(balance)
            }
        }
        Ok(())
    }
    pub(crate) async fn register_new_identity(
        &mut self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<(), Error> {
        let Some(wallet) = self.loaded_wallet.as_ref() else {
            return Ok(());
        };

        //// Core steps

        // first we create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (asset_lock_transaction, asset_lock_proof_private_key, mut maybe_asset_lock_proof) =
            if let Some((
                asset_lock_transaction,
                asset_lock_proof_private_key,
                maybe_asset_lock_proof,
            )) = &self.identity_asset_lock_private_key_in_creation
            {
                (
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key.clone(),
                    maybe_asset_lock_proof.clone(),
                )
            } else {
                let (asset_lock_transaction, asset_lock_proof_private_key) =
                    wallet.registration_transaction(None, amount)?;

                self.identity_asset_lock_private_key_in_creation = Some((
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key.clone(),
                    None,
                ));

                self.save();

                (asset_lock_transaction, asset_lock_proof_private_key, None)
            };

        // let block_hash: Vec<u8> = (GetStatusRequest {})
        //     .execute(dapi_client, RequestSettings::default())
        //     .await
        //     .map_err(|e| RegisterIdentityError(e.to_string()))?
        //     .chain
        //     .map(|chain| chain.best_block_hash)
        //     .ok_or_else(|| RegisterIdentityError("missing `chain` field".to_owned()))?;

        // let core_transactions_stream = TransactionsWithProofsRequest {
        //     bloom_filter: Some(bloom_filter_proto),
        //     count: 5,
        //     send_transaction_hashes: false,
        //     from_block: Some(FromBlock::FromBlockHash(block_hash)),
        // }
        //     .execute(dapi_client, RequestSettings::default())
        //     .await
        //     .map_err(|e| RegisterIdentityError(e.to_string()))?;

        let asset_lock_proof = if let Some(asset_lock_proof) = maybe_asset_lock_proof {
            asset_lock_proof.clone()
        } else {
            let asset_lock = Self::broadcast_and_retrieve_asset_lock(sdk, &asset_lock_transaction)
                .await
                .map_err(|e| {
                    Error::SdkExplainedError("error broadcasting transaction".to_string(), e.into())
                })?;
            self.identity_asset_lock_private_key_in_creation = Some((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key.clone(),
                Some(asset_lock.clone()),
            ));
            self.save();
            asset_lock
        };

        //// Platform steps

        let mut std_rng = StdRng::from_entropy();

        let (identity, keys): (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) =
            Identity::random_identity_with_main_keys_with_private_key(
                2,
                &mut std_rng,
                sdk.version(),
            )?;

        let mut signer = SimpleSigner::default();

        signer.add_keys(keys);

        let updated_identity = identity
            .put_to_platform_and_wait_for_response(
                sdk,
                asset_lock_proof.clone(),
                &asset_lock_proof_private_key,
                &signer,
            )
            .await?;

        self.loaded_identity = Some(updated_identity);

        self.save();

        Ok(())
    }

    pub(crate) async fn broadcast_and_retrieve_asset_lock(
        sdk: &Sdk,
        asset_lock_transaction: &Transaction,
    ) -> Result<AssetLockProof, dash_platform_sdk::Error> {
        let asset_lock_stream = sdk.start_instant_send_lock_stream(asset_lock_transaction.txid()).await?;

        // we need to broadcast the transaction to core
        let request = BroadcastTransactionRequest {
            transaction: asset_lock_transaction.serialize(), // transaction but how to encode it as bytes?,
            allow_high_fees: false,
            bypass_limits: false,
        };

        let broadcast_result = sdk.execute(request, RequestSettings::default()).await;

        let asset_lock = if let Err(broadcast_error) = broadcast_result {
            if broadcast_error.to_string().contains("AlreadyExists") {
                let request = GetTransactionRequest {
                    id: asset_lock_transaction.txid().to_string(),
                };

                let transaction_info = sdk.execute(request, RequestSettings::default()).await?;

                if transaction_info.is_chain_locked {
                    // it already exists, just return an asset lock
                    AssetLockProof::Chain(ChainAssetLockProof {
                        core_chain_locked_height: transaction_info.height,
                        out_point: OutPoint {
                            txid: asset_lock_transaction.txid(),
                            vout: 0,
                        },
                    })
                } else {
                    return Err(broadcast_error.into());
                }
            } else {
                return Err(broadcast_error.into());
            }
        } else {
            Sdk::wait_for_asset_lock_proof_for_transaction(
                asset_lock_stream,
                asset_lock_transaction,
                Some(5 * 60000),
            )
            .await?
        };

        Ok(asset_lock)
    }
}
