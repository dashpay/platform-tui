use std::{str::FromStr, sync::RwLock};

use bincode::{
    de::{BorrowDecoder, Decoder},
    enc::Encoder,
    error::{DecodeError, EncodeError},
    BorrowDecode, Decode, Encode,
};
use dashcore::{
    secp256k1::Secp256k1, Address, Network, OutPoint, PrivateKey, ScriptBuf, Transaction, TxOut,
};

use crate::{app::error::Error::InsightError, managers::insight::utxos_with_amount_for_addresses};

#[derive(Debug, Clone, Encode, Decode)]
pub enum Wallet {
    SingleKeyWallet(SingleKeyWallet),
}

impl Wallet {
    pub fn registration_transaction(&self, seed: Option<u64>, amount: u64) -> Result<(Transaction, PrivateKey), Error> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let random_private_key : [u8;32] = rng.gen();
        let private_key = PrivateKey::from_slice(&random_private_key, Network::Testnet).expect("expected a private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);

        let one_time_key_hash = public_key.pubkey_hash();

        let (utxos, change) = self.take_unspent_utxos_for(amount).ok_or(Error::WalletError("Not enough balance in wallet".to_string()))?;

        let address = self.change_address();

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
            script_pubkey: ScriptBuf::new_p2pkh(&public_key_hash),
        };
        let payload = AssetLockPayload {
            version: 0,
            credit_outputs: vec![payload_output],
        };

        let mut writer = LegacySighash::engine();
        let input_index = 0;
        let script_pubkey = dashcore::ScriptBuf::new();
        let sighash_u32 = 0u32;

        let tx: Transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![input],
            output: vec![burn_output, change_output],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };
        let cache = SighashCache::new(&tx);
        let result = cache.legacy_encode_signing_data_to(&mut writer, input_index, &script_pubkey, sighash_u32)
                .is_sighash_single_bug()
                .expect("writer can't fail");



        (, random_private_key)
    }

    pub fn change_address(&self) -> Address {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                wallet.change_address()
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                format!(
                    "Single Key Wallet \npublic key: {} \naddress: {} \nbalance: {}",
                    hex::encode(wallet.public_key.as_slice()),
                    wallet.address.as_str(),
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

    pub fn take_unspent_utxos_for(&self, amount: u64) -> Option<(Vec<(OutPoint, TxOut)>, u64)> {
        match self {
            Wallet::SingleKeyWallet(wallet) => wallet.take_unspent_utxos_for(amount),
        }
    }

    pub async fn reload_utxos(&self) {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                let Ok(utxos) =
                    utxos_with_amount_for_addresses(&[wallet.address.as_str()], false).await
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
    pub private_key: [u8; 32],
    pub public_key: Vec<u8>,
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
        self.private_key.as_slice().encode(encoder)?;
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
        let bytes: [u8; 32] = Vec::<u8>::decode(decoder)?.try_into().unwrap();
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
            private_key: private_key.inner.secret_bytes(),
            public_key: public_key.to_bytes(),
            address,
            utxos: RwLock::new(utxos),
        })
    }
}

impl<'a> BorrowDecode<'a> for SingleKeyWallet {
    fn borrow_decode<D: BorrowDecoder<'a>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let bytes: [u8; 32] = Vec::<u8>::decode(decoder)?.try_into().unwrap();
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
            private_key: private_key.inner.secret_bytes(),
            public_key: public_key.to_bytes(),
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

    pub fn take_unspent_utxos_for(&self, amount: u64) -> Option<(Vec<(OutPoint, TxOut)>, u64)> {
        let mut utxos = self.utxos.write().unwrap();

        let mut required: i64 = amount as i64;
        let mut taken_utxos = vec![];

        for (outpoint, utxo) in utxos.iter() {
            if required <= 0 {
                break;
            }
            required -= utxo.value as i64;
            taken_utxos.push((outpoint.clone(), utxo.clone()));
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
}
