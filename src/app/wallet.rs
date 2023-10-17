use crate::app::error::Error::InsightError;
use crate::managers::insight::utxos_with_amount_for_addresses;
use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use dashcore::secp256k1::Secp256k1;
use dashcore::{Address, Network, OutPoint, PrivateKey, ScriptBuf, Transaction, TxOut};
use std::str::FromStr;
use std::sync::RwLock;

#[derive(Debug, Clone, Encode, Decode)]
pub enum Wallet {
    SingleKeyWallet(SingleKeyWallet),
}

impl Wallet {
    pub fn registration_transaction(&self) -> (Transaction, PrivateKey) {
        todo!()
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

    pub async fn reload_utxos(&self) {
        match self {
            Wallet::SingleKeyWallet(wallet) => {
                let Ok(utxos) = utxos_with_amount_for_addresses(&[wallet.address.as_str()], false).await else {
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
    pub address: String,
    pub utxos: RwLock<Vec<(OutPoint, TxOut)>>,
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
        //todo: make the network be part of state
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
            .collect::<Vec<_>>();

        Ok(SingleKeyWallet {
            private_key: private_key.inner.secret_bytes(),
            public_key: public_key.to_bytes(),
            address: address.to_string(),
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
        //todo: make the network be part of state
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
            .collect::<Vec<_>>();

        Ok(SingleKeyWallet {
            private_key: private_key.inner.secret_bytes(),
            public_key: public_key.to_bytes(),
            address: address.to_string(),
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
}
