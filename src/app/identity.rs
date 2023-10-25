//! Identities backend logic.

use bip37_bloom_filter::{BloomFilter, BloomFilterData};
use dapi_grpc::{
    core::v0::{
        self as core_proto, transactions_with_proofs_request::FromBlock,
        transactions_with_proofs_response, InstantSendLockMessages, TransactionsWithProofsResponse,
    },
    platform::v0::{
        self as platform_proto, get_identity_response::Result as ProtoResult, GetIdentityResponse,
    },
};
use dpp::{
    platform_value::string_encoding::Encoding,
    prelude::{Identifier, Identity},
    serialization::PlatformDeserializable,
};
use rs_dapi_client::{DapiClient, DapiRequest, RequestSettings};

use crate::app::{error::Error, state::AppState};

pub(super) async fn fetch_identity_by_b58_id(
    client: &mut DapiClient,
    b58_id: String,
) -> Result<Identity, Error> {
    let identifier = Identifier::from_string(b58_id.as_str(), Encoding::Base58)?;
    let request = platform_proto::GetIdentityRequest {
        id: identifier.to_vec(),
        prove: false,
    };
    let response = request.execute(client, RequestSettings::default()).await;
    if let Ok(GetIdentityResponse {
        result: Some(ProtoResult::Identity(bytes)),
        ..
    }) = response
    {
        Ok(Identity::deserialize_from_bytes(&bytes)?)
    } else {
        Err(Error::DapiError)
    }
}

#[derive(Debug)]
pub struct RegisterIdentityError(String);

impl AppState {
    pub async fn register_identity(
        &mut self,
        dapi_client: &mut DapiClient,
    ) -> Result<(), RegisterIdentityError> {
        let Some(wallet) = self.loaded_wallet.as_ref() else {
            return Ok(());
        };

        //// Core steps

        // first we create the wallet registration transaction, this locks funds that we
        // can transfer from core to platform
        let (transaction, private_key) = wallet.registration_transaction();

        self.identity_creation_private_key = Some(private_key.inner.secret_bytes());

        // create the bloom filter

        let bloom_filter = BloomFilter::builder(todo!(), 0.0001)
            .expect("this FP rate allows up to 10000 items")
            .add_element(todo!()) // TODO add transactions and set the n_elements accordingly
            .add_element(todo!())
            .build();

        let bloom_filter_proto = {
            let BloomFilterData {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            } = bloom_filter.into();
            core_proto::BloomFilter {
                v_data,
                n_hash_funcs,
                n_tweak,
                n_flags,
            }
        };
        // let bloom_filter = wallet.bloom_filter() todo() -> Sam

        // we should subscribe and listen to transactions from core todo() -> Evgeny

        let block_hash: Vec<u8> = (core_proto::GetStatusRequest {})
            .execute(dapi_client, RequestSettings::default())
            .await
            .map_err(|e| RegisterIdentityError(e.to_string()))?
            .chain
            .map(|chain| chain.best_block_hash)
            .ok_or_else(|| RegisterIdentityError("missing `chain` field".to_owned()))?;

        let core_transactions_stream = core_proto::TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter_proto),
            count: 0,
            send_transaction_hashes: false,
            from_block: Some(FromBlock::FromBlockHash(block_hash)),
        }
        .execute(dapi_client, RequestSettings::default())
        .await
        .map_err(|e| RegisterIdentityError(e.to_string()))?;

        // we need to broadcast the transaction to core todo() -> Evgeny
        core_proto::BroadcastTransactionRequest {
            transaction: todo!(), // transaction but how to encode it as bytes?,
            allow_high_fees: false,
            bypass_limits: false,
        }
        .execute(&mut dapi_client, RequestSettings::default())
        .await
        .map_err(|e| RegisterIdentityError(e.to_string()))?;

        // Get the instant send lock back todo() -> Evgeny
        // Here we intentionally block our UI for now
        let mut instant_send_lock_messages =
            wait_for_instant_send_lock_messages(core_transactions_stream).await?;

        //// Platform steps

        // Create the identity create state transition todo() -> Sam

        // Subscribe to state transition result todo() -> Evgeny
        let state_transition_proof = platform_proto::WaitForStateTransitionResultRequest {
            state_transition_hash: todo!(),
            prove: true,
        }
        .execute(dapi_client, RequestSettings::default())
        .await
        .map_err(|e| RegisterIdentityError(e.to_string()))?;

        // Through sdk send this transaction and get back proof that the identity was
        // created todo() -> Evgeny
        platform_proto::BroadcastStateTransitionRequest {
            state_transition: todo!(),
        }
        .execute(dapi_client, RequestSettings::default())
        .await
        .map_err(|e| RegisterIdentityError(e.to_string()))?;

        // Verify proof and get identity todo() -> Sam

        // Add Identity as the current identity in the state todo() -> Sam

        Ok(())
    }
}

async fn wait_for_instant_send_lock_messages(
    mut stream: rs_dapi_client::tonic::Streaming<TransactionsWithProofsResponse>,
) -> Result<InstantSendLockMessages, RegisterIdentityError> {
    let instant_send_lock_messages;
    loop {
        if let Some(TransactionsWithProofsResponse { responses }) = stream
            .message()
            .await
            .map_err(|e| RegisterIdentityError(e.to_string()))?
        {
            match responses {
                Some(transactions_with_proofs_response::Responses::InstantSendLockMessages(
                    messages,
                )) => {
                    instant_send_lock_messages = messages;
                    break;
                }
                _ => continue,
            }
        } else {
            return Err(RegisterIdentityError(
                "steam closed unexpectedly".to_owned(),
            ));
        }
    }

    Ok(instant_send_lock_messages)
}
