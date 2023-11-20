//! Identities backend logic.

use dapi_grpc::core::v0::{
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetTransactionRequest,
};
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::{get_identity_balance_request, GetIdentityBalanceRequest};
use dash_platform_sdk::platform::Fetch;
use std::collections::BTreeMap;

use crate::backend::error::Error;
use crate::backend::info_display::InfoDisplay;
use crate::backend::{stringify_result, stringify_result_keep_item, AppState, BackendEvent, Task};
use dash_platform_sdk::platform::transition::put_identity::PutIdentity;
use dash_platform_sdk::Sdk;
use dpp::dashcore::psbt::serialize::Serialize;
use dpp::dashcore::{InstantLock, Network, OutPoint, PrivateKey, Transaction};
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::Identifier;
use dpp::prelude::{AssetLockProof, Identity, IdentityPublicKey};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_dapi_client::{Dapi, DapiClientError, RequestSettings};
use simple_signer::signer::SimpleSigner;
use toml::to_string;
use tuirealm::props::{PropValue, TextSpan};

pub(super) fn identity_to_spans(identity: &Identity) -> Result<Vec<PropValue>, Error> {
    let textual = toml::to_string_pretty(identity).expect("identity is serializable");
    Ok(textual
        .lines()
        .map(|line| PropValue::TextSpan(TextSpan::new(line)))
        .collect())
}

pub(super) async fn fetch_identity_by_b58_id(
    sdk: &mut Sdk,
    base58_id: &str,
) -> Result<(Option<Identity>, String), String> {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let id_bytes = Identifier::from_string(base58_id, Encoding::Base58)
        .map_err(|_| "can't parse identifier as base58 string".to_owned())?;

    let fetch_result = Identity::fetch(sdk, id_bytes).await;
    stringify_result_keep_item(fetch_result)
}

#[derive(Clone, PartialEq)]
pub(crate) enum IdentityTask {
    RegisterIdentity(u64),
}

impl AppState {
    pub(crate) async fn run_identity_task(&self, sdk: &Sdk, task: IdentityTask) -> BackendEvent {
        match task {
            IdentityTask::RegisterIdentity(amount) => {
                let result = self
                    .register_new_identity(sdk, amount)
                    .await
                    .map(|identity| identity.display_info(0))
                    .map_err(|e| e.to_string());
                BackendEvent::TaskCompleted {
                    task: Task::Identity(task),
                    execution_result: result,
                }
            }
        }
    }

    pub(crate) async fn refresh_identity_balance(&mut self, sdk: &Sdk) -> Result<(), Error> {
        if let Some(identity) = self.loaded_identity.blocking_lock().as_mut() {
            let balance = u64::fetch(
                sdk,
                GetIdentityBalanceRequest {
                    version: Some(get_identity_balance_request::Version::V0(
                        GetIdentityBalanceRequestV0 {
                            id: identity.id().to_vec(),
                            prove: true,
                        },
                    )),
                },
            )
            .await?;
            if let Some(balance) = balance {
                identity.set_balance(balance)
            }
        }
        Ok(())
    }
    pub(crate) async fn register_new_identity(
        &self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<Identity, Error> {
        // First we need to make the transaction from the wallet
        // We start by getting a lock on the wallet

        let mut loaded_wallet = self.loaded_wallet.lock().await;
        let Some(wallet) = loaded_wallet.as_mut() else {
            return Err(Error::IdentityRegistrationError(
                "No wallet loaded".to_string(),
            ));
        };

        //// Core steps

        // We create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (
            asset_lock_transaction,
            asset_lock_proof_private_key,
            mut maybe_asset_lock_proof,
            mut maybe_identity_info,
        ) = if let Some((
            asset_lock_transaction,
            asset_lock_proof_private_key,
            maybe_asset_lock_proof,
            maybe_identity,
        )) = self
            .identity_asset_lock_private_key_in_creation
            .blocking_lock()
            .as_ref()
        {
            (
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key.clone(),
                maybe_asset_lock_proof.clone(),
                maybe_identity.clone(),
            )
        } else {
            let (asset_lock_transaction, asset_lock_proof_private_key) =
                wallet.registration_transaction(None, amount)?;

            let mut identity_asset_lock_private_key_in_creation = self
                .identity_asset_lock_private_key_in_creation
                .blocking_lock();

            identity_asset_lock_private_key_in_creation.replace((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key.clone(),
                None,
                None,
            ));

            self.save();

            (
                asset_lock_transaction,
                asset_lock_proof_private_key,
                None,
                None,
            )
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

            let mut identity_asset_lock_private_key_in_creation = self
                .identity_asset_lock_private_key_in_creation
                .blocking_lock();

            identity_asset_lock_private_key_in_creation.replace((
                asset_lock_transaction.clone(),
                asset_lock_proof_private_key.clone(),
                Some(asset_lock.clone()),
                None,
            ));

            self.save();
            asset_lock
        };

        //// Platform steps

        let (identity, keys): (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) =
            if let Some(identity_info) = maybe_identity_info {
                identity_info.clone()
            } else {
                let mut std_rng = StdRng::from_entropy();
                let (mut identity, keys): (Identity, BTreeMap<IdentityPublicKey, Vec<u8>>) =
                    Identity::random_identity_with_main_keys_with_private_key(
                        2,
                        &mut std_rng,
                        sdk.version(),
                    )?;
                identity.set_id(
                    asset_lock_proof
                        .create_identifier()
                        .expect("expected to create an identifier"),
                );

                let mut identity_asset_lock_private_key_in_creation = self
                    .identity_asset_lock_private_key_in_creation
                    .blocking_lock();

                identity_asset_lock_private_key_in_creation.replace((
                    asset_lock_transaction.clone(),
                    asset_lock_proof_private_key.clone(),
                    Some(asset_lock_proof.clone()),
                    Some((identity.clone(), keys.clone())),
                ));

                self.save();
                (identity, keys)
            };

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

        if updated_identity.id() != identity.id() {
            panic!("identity ids don't match");
        }

        self.loaded_identity
            .blocking_lock()
            .replace(updated_identity.clone());

        let keys = self
            .identity_asset_lock_private_key_in_creation
            .blocking_lock()
            .take()
            .expect("expected something to be in creation")
            .3
            .expect("expected an identity")
            .1
            .into_iter()
            .map(|(key, private_key)| {
                (
                    (identity.id(), key.id()),
                    PrivateKey::from_slice(private_key.as_slice(), Network::Testnet).unwrap(),
                )
            });

        self.identity_private_keys.blocking_lock().extend(keys);

        self.save();

        Ok(updated_identity)
    }

    pub(crate) async fn broadcast_and_retrieve_asset_lock(
        sdk: &Sdk,
        asset_lock_transaction: &Transaction,
    ) -> Result<AssetLockProof, dash_platform_sdk::Error> {
        let asset_lock_stream = sdk
            .start_instant_send_lock_stream(asset_lock_transaction.txid())
            .await?;

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
