//! Application screens module.

mod add_wallet;
mod contract;
mod get_contract;
mod get_identity;
mod identity;
mod main;
mod shared;
mod version_upgrade;
mod wallet;

pub(crate) use add_wallet::{AddWalletScreen, AddWalletScreenCommands, PrivateKeyInput};
pub(crate) use contract::{ContractScreen, ContractScreenCommands};
pub(crate) use get_contract::{ContractIdInput, GetContractScreen, GetContractScreenCommands};
pub(crate) use get_identity::{GetIdentityScreen, GetIdentityScreenCommands, IdentityIdInput};
pub(crate) use identity::{IdentityScreen, IdentityScreenCommands};
pub(crate) use main::{MainScreen, MainScreenCommands};
pub(crate) use shared::Info;
pub(crate) use version_upgrade::VersionUpgradeCommands;
pub(crate) use wallet::{WalletScreen, WalletScreenCommands};
