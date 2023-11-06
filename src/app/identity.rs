//! Identities backend logic.

use std::collections::BTreeMap;
use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::core::v0::{BroadcastTransactionRequest};

use dash_platform_sdk::Sdk;
use dpp::dashcore::InstantLock;
use dpp::dashcore::psbt::serialize::Serialize;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::{AssetLockProof, Identity, IdentityPublicKey};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_dapi_client::{Dapi, RequestSettings};
use simple_signer::signer::SimpleSigner;

use crate::app::{state::AppState};


#[derive(Debug)]
pub struct RegisterIdentityError(String);

impl AppState {
    pub async fn register_new_identity(
        &mut self,
        sdk: &mut Sdk,
        amount: u64,
    ) -> Result<(), RegisterIdentityError> {
        let Some(wallet) = self.loaded_wallet.as_ref() else {
            return Ok(());
        };

        //// Core steps

        // first we create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (transaction, asset_lock_proof_private_key) = wallet.registration_transaction(None, amount)?;

        self.identity_creation_private_key = Some(asset_lock_proof_private_key.inner.secret_bytes());

        // create the bloom filter

        let bloom_filter = BloomFilter::builder(1, 0.0001)
            .expect("this FP rate allows up to 10000 items")
            .add_element(transaction.txid().as_ref())
            .build();

        let bloom_filter_proto = {
            let BloomFilterData {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            } = bloom_filter.into();
            dapi_grpc::core::v0::BloomFilter {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            }
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

        let asset_lock_stream = sdk.start_instant_send_lock_stream().await?;

        // we need to broadcast the transaction to core
        let request = BroadcastTransactionRequest {
            transaction: transaction.serialize(), // transaction but how to encode it as bytes?,
            allow_high_fees: false,
            bypass_limits: false,
        };

        sdk.execute(request, RequestSettings::default())
            .await
            .map_err(|e| RegisterIdentityError(e.to_string()))?;

        // Get the instant send lock back
        // Here we intentionally block our UI for now
        let mut instant_send_lock_messages =
            wait_for_instant_send_lock_messages(asset_lock_stream).await?;

        // It is possible we didn't get any instant send lock back. In that case we should wait for
        // a chain locked block todo()

        let instant_lock : InstantLock = instant_send_lock_messages.first(); //todo()

        //// Platform steps

        // We need to create the asset lock proof

        let asset_lock_proof = AssetLockProof::Instant(InstantAssetLockProof::new(instant_lock, transaction, 0));

        let mut std_rng = StdRng::from_entropy();

        let (identity, keys) : (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) = Identity::random_identity_with_main_keys_with_private_key(2, &mut std_rng, sdk.version())?;

        let mut signer = SimpleSigner::default();

        signer.add_keys(keys);

        let state_transition_stream = sdk.start_state_transition_stream().await?;

        identity.put_to_platform(sdk, asset_lock_proof, &asset_lock_proof_private_key, &signer).await?;

        Ok(())
    }
}

