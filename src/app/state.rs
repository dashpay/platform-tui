use dpp::prelude::Identity;
use crate::app::wallet::Wallet;

#[derive(Debug, Default)]
pub struct AppState {
    pub loaded_identity : Option<Identity>,
    pub loaded_wallet: Option<Wallet>,
}