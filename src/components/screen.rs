//! Application screens module.

mod add_wallet;
mod choose_document_type;
mod contract;
mod document_type;
mod get_identity;
mod get_system_contract;
mod get_user_contract;
mod identity;
mod main;
pub(crate) mod shared;
mod strategies;
mod strategy_confirm;
mod strategy_contracts;
mod strategy_create;
mod strategy_identity_inserts;
mod strategy_load;
mod strategy_operations;
mod strategy_start_identities;
mod version_upgrade;
mod wallet;

pub(crate) use add_wallet::AddWalletScreen;
pub(crate) use add_wallet::AddWalletScreenCommands;
pub(crate) use add_wallet::PrivateKeyInput;
pub(crate) use choose_document_type::ChooseDocumentTypeScreen;
pub(crate) use choose_document_type::ChooseDocumentTypeScreenCommands;
pub(crate) use contract::ContractScreen;
pub(crate) use contract::ContractScreenCommands;
pub(crate) use document_type::DocumentTypeScreen;
pub(crate) use document_type::DocumentTypeScreenCommands;
pub(crate) use get_identity::GetIdentityScreen;
pub(crate) use get_identity::GetIdentityScreenCommands;
pub(crate) use get_identity::IdentityIdInput;
pub(crate) use get_system_contract::GetSystemContractScreen;
pub(crate) use get_system_contract::GetSystemContractScreenCommands;
pub(crate) use get_user_contract::GetUserContractScreen;
pub(crate) use get_user_contract::GetUserContractScreenCommands;
pub(crate) use get_user_contract::UserContractIdInput;
pub(crate) use identity::IdentityScreen;
pub(crate) use identity::IdentityScreenCommands;
pub(crate) use main::MainScreen;
pub(crate) use main::MainScreenCommands;
pub(crate) use strategies::StrategiesScreen;
pub(crate) use strategies::StrategiesScreenCommands;
pub(crate) use strategies::StrategySelect;
pub(crate) use strategy_confirm::ConfirmStrategyScreen;
pub(crate) use strategy_confirm::ConfirmStrategyScreenCommands;
pub(crate) use strategy_contracts::AddContractStruct;
pub(crate) use strategy_contracts::StrategyContractsScreen;
pub(crate) use strategy_contracts::StrategyContractsScreenCommands;
pub(crate) use strategy_create::CreateStrategyScreen;
pub(crate) use strategy_create::CreateStrategyScreenCommands;
pub(crate) use strategy_identity_inserts::StrategyIdentityInsertsScreen;
pub(crate) use strategy_identity_inserts::StrategyIdentityInsertsScreenCommands;
pub(crate) use strategy_load::DeleteStrategyStruct;
pub(crate) use strategy_load::LoadStrategyScreen;
pub(crate) use strategy_load::LoadStrategyScreenCommands;
pub(crate) use strategy_load::LoadStrategyStruct;
pub(crate) use strategy_load::RenameStrategyStruct;
pub(crate) use strategy_operations::ContractUpdateStruct;
pub(crate) use strategy_operations::DocumentStruct;
pub(crate) use strategy_operations::FrequencyStruct;
pub(crate) use strategy_operations::IdentityUpdateStruct;
pub(crate) use strategy_operations::SelectOperationTypeStruct;
pub(crate) use strategy_operations::StrategyOperationsScreen;
pub(crate) use strategy_operations::StrategyOperationsScreenCommands;
pub(crate) use strategy_start_identities::StartIdentitiesStruct;
pub(crate) use strategy_start_identities::StrategyStartIdentitiesScreen;
pub(crate) use strategy_start_identities::StrategyStartIdentitiesScreenCommands;
pub(crate) use version_upgrade::VersionUpgradeCommands;
pub(crate) use wallet::WalletScreen;
pub(crate) use wallet::WalletScreenCommands;
