use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
    str::FromStr,
    time::Duration,
};

use bincode::{
    de::{BorrowDecoder, Decoder},
    enc::Encoder,
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};
use dapi_grpc::core::v0::{
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetTransactionRequest,
    GetTransactionResponse,
};
use dash_sdk::{RequestSettings, Sdk};
use dash_sdk::dashcore_rpc::{Client, RpcApi};
use dpp::dashcore::secp256k1::SecretKey;
use dpp::dashcore::{
    hashes::Hash,
    psbt::serialize::Serialize,
    secp256k1::{Message, Secp256k1},
    sighash::SighashCache,
    transaction::special_transaction::{asset_lock::AssetLockPayload, TransactionPayload},
    Address, OutPoint, PrivateKey, PublicKey, ScriptBuf, Transaction, TxIn, TxOut, Witness,
};
use rand::{prelude::StdRng, Rng, SeedableRng};
use rs_dapi_client::DapiRequestExecutor;
use tokio::sync::{Mutex, MutexGuard};

use super::{set_clipboard, AppStateUpdate, BackendEvent, CompletedTaskPayload, Task};
use crate::backend::Wallet::SingleKeyWallet as BackendWallet;
use crate::{
    backend::insight::{InsightAPIClient, InsightError},
    config::Config,
};

#[derive(Debug, Clone, PartialEq)]
pub enum WalletTask {
    AddByPrivateKey(String),
    AddRandomKey,
    Refresh,
    CopyAddress,
    ClearLoadedWallet,
    SplitUTXOs(u32),
}

pub async fn add_wallet_by_private_key_as_string<'s>(
    wallet_state: &Mutex<Option<Wallet>>,
    private_key: &String,
    insight: &'s InsightAPIClient,
    core_client: &'s Client,
) -> Result<(), WalletError> {
    let private_key = match private_key.len() {
        64 => {
            // hex
            let bytes = match hex::decode(private_key) {
                Ok(bytes) => bytes,
                Err(_) => return Err(WalletError::Custom("Failed to decode hex".to_string())),
            };
            let network = Config::load().core_network();
            match PrivateKey::from_slice(bytes.as_slice(), network) {
                Ok(key) => key,
                Err(_) => return Err(WalletError::Custom("Expected private key".to_string())),
            }
        }
        51 | 52 => match PrivateKey::from_wif(private_key) {
            Ok(key) => key,
            Err(_) => return Err(WalletError::Custom("Expected WIF key".to_string())),
        },
        _ => {
            return Err(WalletError::Custom(
                "Private key in env file can't be decoded".to_string(),
            ));
        }
    };
    Ok(add_wallet_by_private_key(wallet_state, private_key, insight, core_client).await)
}

pub async fn add_wallet_by_private_key<'s>(
    wallet_state: &'s Mutex<Option<Wallet>>,
    private_key: PrivateKey,
    insight: &'s InsightAPIClient,
    core_client: &Client,
) {
    let secp = Secp256k1::new();
    let public_key = private_key.public_key(&secp);
    let network = Config::load().core_network();
    let address = Address::p2pkh(&public_key, network);
    let mut wallet = Wallet::SingleKeyWallet(SingleKeyWallet {
        private_key,
        public_key,
        address,
        utxos: Default::default(),
    });

    match wallet.reload_utxos(insight, core_client).await {
        Ok(utxos) => match wallet {
            BackendWallet(ref mut single_key_wallet) => {
                single_key_wallet.utxos = utxos;
            }
        },
        Err(_) => {
            // nothing
        }
    };

    let mut wallet_guard = wallet_state.lock().await;
    *wallet_guard = Some(wallet);
}

pub(super) async fn run_wallet_task<'s>(
    sdk: &Sdk,
    wallet_state: &'s Mutex<Option<Wallet>>,
    task: WalletTask,
    insight: &'s InsightAPIClient,
    core_client: &'s Client,
) -> BackendEvent<'s> {
    match task {
        WalletTask::AddByPrivateKey(ref private_key) => {
            match add_wallet_by_private_key_as_string(&wallet_state, private_key, insight, core_client).await {
                Ok(_) => {
                    let wallet_guard = wallet_state.lock().await;
                    let loaded_wallet_update = MutexGuard::map(wallet_guard, |opt| {
                        opt.as_mut().expect("wallet was set above")
                    });

                    BackendEvent::TaskCompletedStateChange {
                        task: Task::Wallet(task),
                        execution_result: Ok("Added wallet".into()),
                        app_state_update: AppStateUpdate::LoadedWallet(loaded_wallet_update),
                    }
                }
                Err(e) => BackendEvent::TaskCompleted {
                    task: Task::Wallet(task),
                    execution_result: Err(format!("{e}")),
                },
            }
        }
        WalletTask::AddRandomKey => {
            let mut rng = StdRng::from_entropy();
            let network = Config::load().core_network();
            let private_key = PrivateKey::new(SecretKey::new(&mut rng), network);
            add_wallet_by_private_key(&wallet_state, private_key, insight, core_client).await;

            let wallet_guard = wallet_state.lock().await;
            let loaded_wallet_update = MutexGuard::map(wallet_guard, |opt| {
                opt.as_mut().expect("wallet was set above")
            });

            BackendEvent::TaskCompletedStateChange {
                task: Task::Wallet(task),
                execution_result: Ok("Added wallet".into()),
                app_state_update: AppStateUpdate::LoadedWallet(loaded_wallet_update),
            }
        }
        WalletTask::Refresh => {
            let mut wallet_guard = wallet_state.lock().await;
            if let Some(wallet) = wallet_guard.deref_mut() {
                match wallet.reload_utxos(insight, core_client).await {
                    Ok(_) => {
                        let loaded_wallet_update = MutexGuard::map(wallet_guard, |opt| {
                            opt.as_mut().expect("wallet was set above")
                        });
                        BackendEvent::TaskCompletedStateChange {
                            task: Task::Wallet(task),
                            execution_result: Ok("Refreshed wallet".into()),
                            app_state_update: AppStateUpdate::LoadedWallet(loaded_wallet_update),
                        }
                    }
                    Err(err) => BackendEvent::TaskCompleted {
                        task: Task::Wallet(task),
                        execution_result: Err(err),
                    },
                }
            } else {
                BackendEvent::TaskCompleted {
                    task: Task::Wallet(task),
                    execution_result: Err(format!("No wallet loaded")),
                }
            }
        }
        WalletTask::CopyAddress => {
            let wallet_guard = wallet_state.lock().await;
            if let Some(wallet) = wallet_guard.deref() {
                let address = wallet.receive_address();
                if set_clipboard(address.to_string()).await.is_ok() {
                    BackendEvent::TaskCompleted {
                        task: Task::Wallet(task),
                        execution_result: Ok("Copied Address".into()),
                    }
                } else {
                    BackendEvent::TaskCompleted {
                        task: Task::Wallet(task),
                        execution_result: Err("Clipboard is not supported".into()),
                    }
                }
            } else {
                BackendEvent::TaskCompleted {
                    task: Task::Wallet(task),
                    execution_result: Err(format!("No wallet loaded")),
                }
            }
        }
        WalletTask::ClearLoadedWallet => {
            let mut wallet_guard = wallet_state.lock().await;
            *wallet_guard = None;
            BackendEvent::TaskCompletedStateChange {
                task: Task::Wallet(task),
                execution_result: Ok(CompletedTaskPayload::String(
                    "Cleared loaded wallet".to_string(),
                )),
                app_state_update: AppStateUpdate::ClearedLoadedWallet,
            }
        }
        WalletTask::SplitUTXOs(count) => {
            let mut wallet_guard = wallet_state.lock().await;
            if let Some(wallet) = &mut *wallet_guard {
                match wallet {
                    Wallet::SingleKeyWallet(sk_wallet) => {
                        match sk_wallet.split_utxos(sdk, count as usize, insight, core_client).await {
                            Ok(_) => BackendEvent::TaskCompleted {
                                task: Task::Wallet(task),
                                execution_result: Ok("Split UTXOs".into()),
                            },
                            Err(_) => BackendEvent::TaskCompleted {
                                task: Task::Wallet(task),
                                execution_result: Err("Failed to split UTXOS properly".into()),
                            },
                        }
                    }
                }
            } else {
                BackendEvent::TaskCompleted {
                    task: Task::Wallet(task),
                    execution_result: Err(format!("No wallet loaded")),
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error(transparent)]
    Insight(InsightError),
    #[error("not enough balance")]
    Balance,
    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum Wallet {
    SingleKeyWallet(SingleKeyWallet),
}

impl Wallet {
    pub(crate) fn private_key_for_address(&self, address: &Address) -> &PrivateKey {
        match self {
            Wallet::SingleKeyWallet(single_wallet) => {
                single_wallet.private_key_for_address(address)
            }
        }
    }

    pub(crate) fn asset_lock_transaction(
        &mut self,
        seed: Option<u64>,
        amount: u64,
    ) -> Result<(Transaction, PrivateKey), WalletError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let fee = 30_000;
        let random_private_key: [u8; 32] = rng.gen();
        let network = Config::load().core_network();
        let private_key =
            PrivateKey::from_slice(&random_private_key, network).expect("expected a private key");

        let secp = Secp256k1::new();
        let asset_lock_public_key = private_key.public_key(&secp);

        let one_time_key_hash = asset_lock_public_key.pubkey_hash();

        let (mut utxos, change) =
            self.take_unspent_utxos_for(amount + fee)
                .ok_or(WalletError::Custom(
                    "take_unspent_utxos_for() returned None".to_string(),
                ))?;

        let change_address = self.change_address();

        let payload_output = TxOut {
            value: amount,
            script_pubkey: ScriptBuf::new_p2pkh(&one_time_key_hash),
        };
        let burn_output = TxOut {
            value: amount,
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };
        if change < fee {
            return Err(WalletError::Custom(
                "Change < Fee in asset_lock_transaction()".to_string(),
            ));
        }
        let change_output = TxOut {
            value: change - fee,
            script_pubkey: change_address.script_pubkey(),
        };
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![payload_output],
        };

        // we need to get all inputs from utxos to add them to the transaction

        let inputs = utxos
            .iter()
            .map(|(utxo, _)| {
                let mut tx_in = TxIn::default();
                tx_in.previous_output = utxo.clone();
                tx_in
            })
            .collect();

        let sighash_u32 = 1u32;

        let mut tx: Transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: inputs,
            output: vec![burn_output, change_output],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };

        let cache = SighashCache::new(&tx);

        // Next, collect the sighashes for each input since that's what we need from the
        // cache
        let sighashes: Vec<_> = tx
            .input
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let script_pubkey = utxos
                    .get(&input.previous_output)
                    .expect("expected a txout")
                    .0
                    .script_pubkey
                    .clone();
                cache
                    .legacy_signature_hash(i, &script_pubkey, sighash_u32)
                    .expect("expected sighash")
            })
            .collect();

        // Now we can drop the cache to end the immutable borrow
        drop(cache);

        tx.input
            .iter_mut()
            .zip(sighashes.into_iter())
            .for_each(|(input, sighash)| {
                // You need to provide the actual script_pubkey of the UTXO being spent
                let (_, public_key, input_address) = utxos
                    .remove(&input.previous_output)
                    .expect("expected a txout");
                let message =
                    Message::from_slice(sighash.as_byte_array()).expect("Error creating message");

                let private_key = self.private_key_for_address(&input_address);

                // Sign the message with the private key
                let sig = secp.sign_ecdsa(&message, &private_key.inner);

                // Serialize the DER-encoded signature and append the sighash type
                let mut serialized_sig = sig.serialize_der().to_vec();

                let mut sig_script = vec![serialized_sig.len() as u8 + 1];

                sig_script.append(&mut serialized_sig);

                sig_script.push(1);

                let mut serialized_pub_key = public_key.serialize();

                sig_script.push(serialized_pub_key.len() as u8);
                sig_script.append(&mut serialized_pub_key);
                // Create script_sig
                input.script_sig = ScriptBuf::from_bytes(sig_script);
            });

        Ok((tx, private_key))
    }

    pub fn receive_address(&self) -> Address {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.receive_address(),
        }
    }

    pub fn change_address(&self) -> Address {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.change_address(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                format!(
                    "Single Key Wallet \npublic key: {} \naddress: {} \nbalance: {}",
                    hex::encode(wallet.public_key.inner.serialize()),
                    wallet.address.to_string().as_str(),
                    wallet.balance_dash_formatted()
                )
            }
        }
    }

    pub fn balance_dash_formatted(&self) -> String {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.balance_dash_formatted(),
        }
    }

    pub fn balance(&self) -> u64 {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.balance(),
        }
    }

    pub fn take_unspent_utxos_for(
        &mut self,
        amount: u64,
    ) -> Option<(BTreeMap<OutPoint, (TxOut, PublicKey, Address)>, u64)> {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.take_unspent_utxos_for(amount),
        }
    }

    pub async fn reload_utxos(
        &mut self,
        insight: &InsightAPIClient,
        core_client: &Client,
    ) -> Result<HashMap<OutPoint, TxOut>, String> {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.reload_utxos(insight, core_client).await,
        }
    }
}

#[derive(Debug)]
pub struct SingleKeyWallet {
    pub private_key: PrivateKey,
    pub public_key: PublicKey,
    pub address: Address,
    pub utxos: HashMap<OutPoint, TxOut>,
}

impl Clone for SingleKeyWallet {
    fn clone(&self) -> Self {
        Self {
            private_key: self.private_key,
            public_key: self.public_key.clone(),
            address: self.address.clone(),
            utxos: self.utxos.clone(),
        }
    }
}

impl Encode for SingleKeyWallet {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.private_key.inner.as_ref().encode(encoder)?;
        let string_utxos = self
            .utxos
            .iter()
            .map(|(outpoint, txout)| {
                (
                    outpoint.to_string(),
                    txout.value,
                    hex::encode(txout.script_pubkey.as_bytes()),
                )
            })
            .collect::<Vec<_>>();
        string_utxos.encode(encoder)
    }
}

impl Decode for SingleKeyWallet {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8; 32]>::decode(decoder)?;
        let string_utxos = Vec::<(String, u64, String)>::decode(decoder)?;
        let network = Config::load().core_network();

        let private_key =
            PrivateKey::from_slice(bytes.as_slice(), network).expect("expected private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        let address = Address::p2pkh(&public_key, network);

        let utxos = string_utxos
            .iter()
            .map(|(outpoint, value, script)| {
                let script = ScriptBuf::from_hex(script)
                    .map_err(|_| {
                        InsightError(format!(
                            "Invalid scriptPubKey format from load of {}",
                            script
                        ))
                    })
                    .unwrap();
                (
                    OutPoint::from_str(outpoint).expect("expected valid outpoint"),
                    TxOut {
                        value: *value,
                        script_pubkey: script,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(SingleKeyWallet {
            private_key,
            public_key,
            address,
            utxos,
        })
    }
}

impl<'a> BorrowDecode<'a> for SingleKeyWallet {
    fn borrow_decode<D: BorrowDecoder<'a>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8; 32]>::decode(decoder)?;
        let string_utxos = Vec::<(String, u64, String)>::decode(decoder)?;
        let network = Config::load().core_network();

        let private_key =
            PrivateKey::from_slice(bytes.as_slice(), network).expect("expected private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        // todo: make the network be part of state
        let address = Address::p2pkh(&public_key, network);

        let utxos = string_utxos
            .iter()
            .map(|(outpoint, value, script)| {
                let script = ScriptBuf::from_hex(script)
                    .map_err(|_| {
                        InsightError(format!(
                            "Invalid scriptPubKey format from load of {}",
                            script
                        ))
                    })
                    .unwrap();
                (
                    OutPoint::from_str(outpoint).expect("expected valid outpoint"),
                    TxOut {
                        value: *value,
                        script_pubkey: script,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(SingleKeyWallet {
            private_key,
            public_key,
            address,
            utxos,
        })
    }
}

impl SingleKeyWallet {
    pub fn balance_dash_formatted(&self) -> String {
        let satoshis = self.balance();
        let dash = satoshis as f64 / 100_000_000f64;
        format!("{:.4}", dash)
    }

    pub fn balance(&self) -> u64 {
        self.utxos.iter().map(|(_, out)| out.value).sum()
    }

    pub fn take_unspent_utxos_for(
        &mut self,
        amount: u64,
    ) -> Option<(BTreeMap<OutPoint, (TxOut, PublicKey, Address)>, u64)> {
        let mut required: i64 = amount as i64;
        let mut taken_utxos = BTreeMap::new();

        for (outpoint, utxo) in self.utxos.iter() {
            if required <= 0 {
                break;
            }
            required -= utxo.value as i64;
            taken_utxos.insert(
                outpoint.clone(),
                (utxo.clone(), self.public_key, self.address.clone()),
            );
        }

        // If we didn't gather enough UTXOs to cover the required amount
        if required > 0 {
            return None;
        }

        // Remove taken UTXOs from the original list
        for (outpoint, _) in &taken_utxos {
            self.utxos.remove(outpoint);
        }

        Some((taken_utxos, required.abs() as u64))
    }

    pub async fn reload_utxos(
        &mut self,
        insight: &InsightAPIClient,
        core_client: &Client,
    ) -> Result<HashMap<OutPoint, TxOut>, String> {
        // First, let's try to get UTXOs from the RPC client using `list_unspent`.
        match core_client
            .list_unspent(Some(1), None, Some(&[&self.address]), None, None)
        {
            Ok(utxos) => {
                // Test log statement
                tracing::info!("{:?} utxos", utxos.len());

                // Convert RPC UTXOs to the desired HashMap format
                let mut utxo_map = HashMap::new();
                for utxo in utxos {
                    let outpoint = OutPoint::new(utxo.txid, utxo.vout);
                    let tx_out = TxOut {
                        value: utxo.amount.to_sat(),
                        script_pubkey: utxo.script_pub_key,
                    };
                    utxo_map.insert(outpoint, tx_out);
                }
                self.utxos = utxo_map.clone(); // Store the result in `self.utxos`
                Ok(utxo_map)
            }
            Err(first_error) => {
                // If that doesn't work, use the Insight API as a fallback
                match insight
                    .utxos_with_amount_for_addresses(&[&self.address])
                    .await
                {
                    Ok(utxos) => {
                        self.utxos = utxos.clone();
                        Ok(utxos)
                    }
                    Err(err) => Err(format!("First error from Core: {}, Second Error from Insight: {}", first_error.to_string(), err.to_string())),
                }
            }
        }
    }

    /// Takes a usize `desired_utxo_count` specifying the desired number of UTXOs one wants the wallet to have
    /// and splits the existing utxos into that many (minus one) equally-valued UTXOs plus one UTXO holding leftover value.
    ///
    /// It does so by executing transactions with itself as the sender and receiver, and
    /// the existing UTXOs as the inputs and just having `desired_utxo_count` outputs. Since Dash Core only
    /// allows 24 outputs per transaction, we have to create (`desired_utxo_count` / 24) transactions.
    ///
    /// Each new UTXO is given a value of ((current_wallet_balance - fee) / desired_utxo_count) where fee is set to 1_000_000_000
    ///
    /// Newly created UTXOs are then used for the creation of more UTXOs
    pub async fn split_utxos(
        &mut self,
        sdk: &Sdk,
        desired_utxo_count: usize,
        insight: &InsightAPIClient,
        core_client: &Client,
    ) -> Result<(), WalletError> {
        tracing::info!("Splitting wallet UTXOs into {} UTXOs", desired_utxo_count);

        // Initialize
        const MAX_OUTPUTS_PER_TRANSACTION: usize = 24; // Dash Core only allows 24 outputs per tx
        let current_wallet_balance = self.balance();
        let mut remaining_utxos_in_wallet = self
            .reload_utxos(insight, core_client)
            .await
            .expect("Expected to reload utxos");
        let mut num_utxos_remaining_to_create = desired_utxo_count;

        // Say we want 50 UTXOs, we need 3 transactions (24 + 24 + 2)
        let number_of_transactions =
            (desired_utxo_count as f64 / MAX_OUTPUTS_PER_TRANSACTION as f64).ceil() as usize;

        // Amount to fund each UTXO.
        // Reserve a buffer of 1_000_000 duffs to make sure we never go over the current balance
        let utxo_split_value = (current_wallet_balance - 1_000_000) / desired_utxo_count as u64;

        // Create and execute the transactions
        tracing::info!("We want to make {} transactions", number_of_transactions);
        for i in 0..number_of_transactions {
            // Number of UTXOs to create with this tx
            let num_utxos_to_create_this_tx =
                std::cmp::min(MAX_OUTPUTS_PER_TRANSACTION, num_utxos_remaining_to_create);

            // Set the outputs of the transaction
            // Only loop num_utxos_to_create-1 times because we'll add an output for excess change at the end (the value will be different)
            let mut new_outputs: Vec<TxOut> = Vec::with_capacity(num_utxos_to_create_this_tx);
            for _ in 0..num_utxos_to_create_this_tx - 1 {
                new_outputs.push(TxOut {
                    value: utxo_split_value,
                    script_pubkey: self.receive_address().script_pubkey(),
                });
            }

            // Select existing UTXOs from the wallet to be the inputs for the transaction
            let mut total_value_of_tx_inputs: u64 = 0;
            let mut selected_utxos = Vec::new();
            let mut remaining_utxos_in_wallet_vec: Vec<(&OutPoint, &TxOut)> =
                remaining_utxos_in_wallet.iter().collect();
            remaining_utxos_in_wallet_vec.sort_by_key(|&(_, txout)| txout.value);
            remaining_utxos_in_wallet_vec.reverse(); // Sort greatest to least value utxo

            for (outpoint, txout) in remaining_utxos_in_wallet_vec.clone() {
                if total_value_of_tx_inputs >= utxo_split_value * num_utxos_to_create_this_tx as u64
                {
                    break; // Selected enough UTXOs for this tx
                }
                total_value_of_tx_inputs += txout.value;
                selected_utxos.push(outpoint.clone());
            }

            // Check if total value of selected UTXOs cover the amount required by new outputs
            if total_value_of_tx_inputs < utxo_split_value * num_utxos_to_create_this_tx as u64 {
                tracing::error!("inputs_total_value < utxo_split_value * current_utxo_count");
                return Err(WalletError::Balance);
            }

            // Add the output for excess change if any
            if total_value_of_tx_inputs
            - (utxo_split_value * (num_utxos_to_create_this_tx - 1) as u64) // Total value of outputs
            > 20_000
            {
                new_outputs.push(TxOut {
                    value: total_value_of_tx_inputs
                        - (utxo_split_value * (num_utxos_to_create_this_tx - 1) as u64) // This is the existing total value of tx outputs
                        - 10_000, // Now inputs should equal outputs, so leave some excess input value to pay the tx fee
                    script_pubkey: self.receive_address().script_pubkey(),
                });
            }

            // Construct the transaction
            let mut tx = Transaction {
                version: 1,
                lock_time: 0,
                input: selected_utxos
                    .iter()
                    .map(|outpoint| TxIn {
                        previous_output: outpoint.clone(),
                        script_sig: ScriptBuf::new(), // Placeholder, will be filled by signing
                        sequence: 0xFFFFFFFF,
                        witness: Witness::new(),
                    })
                    .collect(),
                output: new_outputs,
                special_transaction_payload: None,
            };

            // Sign the transaction
            let secp = Secp256k1::new();
            let cache = SighashCache::new(tx.clone());
            for (i, input) in tx.input.iter_mut().enumerate() {
                let sighash = cache
                    .legacy_signature_hash(
                        i,
                        &self.receive_address().script_pubkey(),
                        1, /* SIGHASH_ALL */
                    )
                    .unwrap();
                let message = Message::from_slice(&sighash[..]).unwrap();
                let sig = secp
                    .sign_ecdsa(&message, &self.private_key.inner)
                    .serialize_der();
                let mut sig_with_sighash = sig.to_vec();
                sig_with_sighash.push(1); // SIGHASH_ALL
                input.script_sig = ScriptBuf::from_bytes(
                    [
                        &[sig_with_sighash.len() as u8], // Convert to slice for uniform handling
                        &sig_with_sighash[..], // Convert Vec<u8> to slice for concatenation
                        &[0x21],               // Single-element slice for the public key length
                        &self.public_key.serialize()[..], // Public key as slice
                    ]
                    .concat(),
                );
            }

            // Attempt to broadcast the transaction
            let request = BroadcastTransactionRequest {
                transaction: tx.serialize(),
                allow_high_fees: false,
                bypass_limits: false,
            };
            let max_retries = 3;
            match sdk.execute(request, RequestSettings::default()).await {
                Ok(BroadcastTransactionResponse { transaction_id: id }) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    let mut retries = 0;
                    let mut transaction_found = false;

                    while retries < max_retries {
                        match sdk
                            .execute(
                                GetTransactionRequest { id: id.clone() },
                                RequestSettings::default(),
                            )
                            .await
                        {
                            Ok(GetTransactionResponse { .. }) => {
                                transaction_found = true;
                                break;
                            }
                            Err(e) => {
                                let error_message = format!("{:?}", e);
                                if error_message.contains("Transaction not found") {
                                    tracing::warn!(
                                        "Transaction not found, retrying... attempt {} of {}",
                                        retries + 1,
                                        max_retries
                                    );
                                    retries += 1;
                                    tokio::time::sleep(Duration::from_secs(3)).await;
                                } else {
                                    tracing::error!(
                                        "Error getting UTXO-splitting transaction: {e}"
                                    );
                                    return Err(WalletError::Balance);
                                }
                            }
                        }
                    }

                    if transaction_found {
                        tracing::info!(
                            "Successfully broadcasted UTXO-splitting transaction {}.",
                            i + 1,
                        );
                    } else {
                        tracing::error!(
                            "Failed to retrieve UTXO-splitting transaction after {} attempts.",
                            max_retries
                        );
                        return Err(WalletError::Balance);
                    }
                }
                Err(error) => {
                    tracing::error!("Transaction broadcast failed: {error}");
                }
            }

            // Update the wallet's UTXO set
            for outpoint in selected_utxos.iter() {
                remaining_utxos_in_wallet.remove(outpoint);
            }
            let txid = tx.txid();
            for (index, output) in tx.output.iter().enumerate() {
                remaining_utxos_in_wallet.insert(
                    OutPoint {
                        txid,
                        vout: index as u32,
                    },
                    output.clone(),
                );
            }

            num_utxos_remaining_to_create -= tx.output.len();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    pub fn change_address(&self) -> Address {
        self.address.clone()
    }

    pub fn receive_address(&self) -> Address {
        self.address.clone()
    }

    pub fn private_key_for_address(&self, address: &Address) -> &PrivateKey {
        if &self.address != address {
            panic!("address doesn't match");
        }
        &self.private_key
    }
}
