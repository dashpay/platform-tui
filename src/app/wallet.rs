use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub enum  Wallet {
    SingleKeyWallet(SingleKeyWallet)
}

#[derive(Debug, Encode, Decode)]
pub struct SingleKeyWallet {
    pub private_key : Vec<u8>,
    pub public_key: Vec<u8>,
    pub balance: u8,
}