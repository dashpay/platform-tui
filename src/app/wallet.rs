use std::collections::{BTreeMap, HashMap};
use std::{str::FromStr, sync::RwLock};

use bincode::{
    de::{BorrowDecoder, Decoder},
    enc::Encoder,
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::psbt::serialize::Serialize;
use dpp::dashcore::secp256k1::Message;
use dpp::dashcore::sighash::SighashCache;
use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::{
    secp256k1::Secp256k1, Address, Network, OutPoint, PrivateKey, PublicKey, Script, ScriptBuf,
    Transaction, TxIn, TxOut,
};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

use crate::app::error::Error;
use crate::{app::error::Error::InsightError, managers::insight::utxos_with_amount_for_addresses};

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
    pub(crate) fn registration_transaction(
        &self,
        seed: Option<u64>,
        amount: u64,
    ) -> Result<(Transaction, PrivateKey), Error> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let random_private_key: [u8; 32] = rng.gen();
        let private_key = PrivateKey::from_slice(&random_private_key, Network::Testnet)
            .expect("expected a private key");

        let secp = Secp256k1::new();
        let asset_lock_public_key = private_key.public_key(&secp);

        let one_time_key_hash = asset_lock_public_key.pubkey_hash();

        let (mut utxos, change) = self
            .take_unspent_utxos_for(amount)
            .ok_or(Error::WalletError(
                "Not enough balance in wallet".to_string(),
            ))?;

        let change_address = self.change_address();

        let burn_output = TxOut {
            value: amount, // 1 Dash
            script_pubkey: ScriptBuf::new_p2pkh(&one_time_key_hash),
        };
        let payload_output = TxOut {
            value: 100000000, // 1 Dash
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };
        let change_output = TxOut {
            value: change,
            script_pubkey: change_address.script_pubkey(),
        };
        let payload = AssetLockPayload {
            version: 0,
            credit_outputs: vec![payload_output],
        };

        // we need to get all inputs from utxos to add them to the transaction

        let mut inputs = utxos
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

        // Next, collect the sighashes for each input since that's what we need from the cache
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
                let mut sig_script = sig.serialize_der().to_vec();

                sig_script.push(sighash_u32 as u8); // Assuming sighash_u32 is something like SIGHASH_ALL (0x01)

                sig_script.append(&mut public_key.serialize());
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
        &self,
        amount: u64,
    ) -> Option<(BTreeMap<OutPoint, (TxOut, PublicKey, Address)>, u64)> {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.take_unspent_utxos_for(amount),
        }
    }

    pub async fn reload_utxos(&self) {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                let Ok(utxos) = utxos_with_amount_for_addresses(&[&wallet.address], false).await
                else {
                    return;
                };
                let mut write_guard = wallet.utxos.write().unwrap();
                *write_guard = utxos;
            }
        }
    }
}

#[derive(Debug)]
pub struct SingleKeyWallet {
    pub private_key: PrivateKey,
    pub public_key: PublicKey,
    pub address: Address,
    pub utxos: RwLock<HashMap<OutPoint, TxOut>>,
}

impl Clone for SingleKeyWallet {
    fn clone(&self) -> Self {
        Self {
            private_key: self.private_key,
            public_key: self.public_key.clone(),
            address: self.address.clone(),
            utxos: RwLock::new(self.utxos.read().unwrap().clone()),
        }
    }
}

impl Encode for SingleKeyWallet {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.private_key.inner.as_ref().encode(encoder)?;
        let utxos = self.utxos.read().unwrap();
        let string_utxos = utxos
            .iter()
            .map(|(outpoint, txout)| {
                (
                    outpoint.to_string(),
                    txout.value,
                    txout.script_pubkey.to_string(),
                )
            })
            .collect::<Vec<_>>();
        string_utxos.encode(encoder)
    }
}

impl Decode for SingleKeyWallet {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8;32]>::decode(decoder)?;
        let string_utxos = Vec::<(String, u64, String)>::decode(decoder)?;

        let private_key = PrivateKey::from_slice(bytes.as_slice(), Network::Testnet)
            .expect("expected private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        // todo: make the network be part of state
        let address = Address::p2pkh(&public_key, Network::Testnet);

        let utxos = string_utxos
            .iter()
            .map(|(outpoint, value, script)| {
                let script = ScriptBuf::from_hex(script)
                    .map_err(|_| InsightError("Invalid scriptPubKey format from load".into()))
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
            utxos: RwLock::new(utxos),
        })
    }
}

impl<'a> BorrowDecode<'a> for SingleKeyWallet {
    fn borrow_decode<D: BorrowDecoder<'a>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes = <[u8;32]>::decode(decoder)?;
        let string_utxos = Vec::<(String, u64, String)>::decode(decoder)?;

        let private_key = PrivateKey::from_slice(bytes.as_slice(), Network::Testnet)
            .expect("expected private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        // todo: make the network be part of state
        let address = Address::p2pkh(&public_key, Network::Testnet);

        let utxos = string_utxos
            .iter()
            .map(|(outpoint, value, script)| {
                let script = ScriptBuf::from_hex(script)
                    .map_err(|_| InsightError("Invalid scriptPubKey format from load".into()))
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
            utxos: RwLock::new(utxos),
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
        let utxos = self.utxos.read().unwrap();
        utxos.iter().map(|(_, out)| out.value).sum()
    }

    pub fn take_unspent_utxos_for(
        &self,
        amount: u64,
    ) -> Option<(BTreeMap<OutPoint, (TxOut, PublicKey, Address)>, u64)> {
        let mut utxos = self.utxos.write().unwrap();

        let mut required: i64 = amount as i64;
        let mut taken_utxos = BTreeMap::new();

        for (outpoint, utxo) in utxos.iter() {
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
            utxos.remove(outpoint);
        }

        Some((taken_utxos, required.abs() as u64))
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
