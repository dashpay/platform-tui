use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub enum  Wallet {
    SingleKeyWallet(SingleKeyWallet)
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct SingleKeyWallet {
    pub private_key : [u8; 32],
    pub public_key: Vec<u8>,
    pub balance: u8,
}