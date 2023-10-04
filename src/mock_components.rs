//! Base (mock) components generic enough to be `MockComponent`, but missing from
//! the standard library of Realm components.

mod command_pallet;
mod completing_input;

pub(crate) use command_pallet::{CommandPallet, CommandPalletKey, KeyType};
pub(crate) use completing_input::{key_event_to_cmd, CompletingInput, HistoryCompletionEngine};
