//! Identities backend logic.

use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::BroadcastTransactionRequest;
use std::collections::BTreeMap;

use dash_platform_sdk::platform::transition::put_identity::PutIdentity;
use dash_platform_sdk::Sdk;
use dpp::dashcore::psbt::serialize::Serialize;
use dpp::dashcore::{InstantLock, Transaction};
use dpp::prelude::{AssetLockProof, Identity, IdentityPublicKey};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_dapi_client::{Dapi, RequestSettings};
use simple_signer::signer::SimpleSigner;

use crate::app::error::Error;
use crate::app::state::AppState;

impl AppState {
    pub async fn register_new_identity(&mut self, sdk: &Sdk, amount: u64) -> Result<(), Error> {
        let Some(wallet) = self.loaded_wallet.as_ref() else {
            return Ok(());
        };

        //// Core steps

        // first we create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (asset_lock_transaction, asset_lock_proof_private_key, asset_lock_proof) = self
            .identity_asset_lock_private_key_in_creation
            .get_or_insert(
                wallet
                    .registration_transaction(None, amount)
                    .map(|(t, k)| (t, k, None))?,
            );

        // create the bloom filter

        // let bloom_filter = BloomFilter::builder(1, 0.0001)
        //     .expect("this FP rate allows up to 10000 items")
        //     .add_element(asset_lock_transaction.txid().as_ref())
        //     .build();
        //
        // let bloom_filter_proto = {
        //     let BloomFilterData {
        //         v_data,
        //         n_hash_funcs,
        //         n_tweak,
        //         n_flags,
        //     } = bloom_filter.into();
        //     dapi_grpc::core::v0::BloomFilter {
        //         v_data,
        //         n_hash_funcs,
        //         n_tweak,
        //         n_flags,
        //     }
        // };

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

        let asset_lock_proof = if let Some(asset_lock_proof) = asset_lock_proof {
            asset_lock_proof.clone()
        } else {
            Self::broadcast_and_retrieve_asset_lock(sdk, asset_lock_transaction).await?
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

        identity
            .put_to_platform(
                sdk,
                asset_lock_proof.clone(),
                &asset_lock_proof_private_key,
                &signer,
            )
            .await?;

        // Wait for the proof that the state transition was properly executed

        Ok(())
    }

    pub async fn broadcast_and_retrieve_asset_lock(
        sdk: &Sdk,
        asset_lock_transaction: &Transaction,
    ) -> Result<AssetLockProof, Error> {
        let asset_lock_stream = sdk.start_instant_send_lock_stream().await?;

        // we need to broadcast the transaction to core
        let request = BroadcastTransactionRequest {
            transaction: asset_lock_transaction.serialize(), // transaction but how to encode it as bytes?,
            allow_high_fees: false,
            bypass_limits: false,
        };

        sdk.execute(request, RequestSettings::default()).await?;

        let asset_lock = Sdk::wait_for_asset_lock_proof_for_transaction(
            asset_lock_stream,
            asset_lock_transaction,
            Some(5 * 60000),
        )
        .await?;

        Ok(asset_lock)
    }
}
