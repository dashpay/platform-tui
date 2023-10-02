//! Application screens module.

mod get_identity;
mod identity;
mod main;
mod contract;

mod get_contract;

pub(crate) use get_identity::GetIdentityScreen;
pub(crate) use get_identity::GetIdentityScreenCommands;
pub(crate) use identity::IdentityScreen;
pub(crate) use identity::IdentityScreenCommands;
pub(crate) use main::MainScreen;
pub(crate) use main::MainScreenCommands;
pub(crate) use contract::ContractScreen;
pub(crate) use contract::ContractScreenCommands;
pub(crate) use get_contract::GetContractScreen;
pub(crate) use get_contract::GetContractScreenCommands;

