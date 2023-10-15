
#[derive(Debug)]
pub enum  Wallet {
    SingleKeyWallet(SingleKeyWallet)
}

#[derive(Debug)]
pub struct SingleKeyWallet {
    pub private_key : Vec<u8>,
    pub public_key: Vec<u8>,
    pub balance: u8,
}