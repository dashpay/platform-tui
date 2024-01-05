//! Contracts views.

mod document_type;
mod fetch_contract;

use std::fmt::{self, Display};

use dpp::{
    data_contract::accessors::v0::DataContractV0Getters, platform_value::string_encoding::Encoding,
    prelude::DataContract,
};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use self::{
    document_type::SelectDocumentTypeFormController,
    fetch_contract::FetchSystemContractScreenController,
};
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent},
    ui::{
        form::{Input, InputStatus, SelectInput},
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 5] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("s", "Fetch system contract"),
    ScreenCommandKey::new("↓ / C-n", "Next contract"),
    ScreenCommandKey::new("↑ / C-p", "Prev contract"),
    ScreenCommandKey::new("Enter", "Select contract"),
];

/// Data contract name (identifier in app state) wrapper for better display
#[derive(Clone)]
struct DataContractEntry {
    name: String,
    id_b58: String,
    document_type_names: Vec<String>,
}

impl DataContractEntry {
    fn new(name: String, contract: &DataContract) -> Self {
        DataContractEntry {
            name,
            id_b58: contract.id_ref().to_string(Encoding::Base58),
            document_type_names: contract.document_types().keys().cloned().collect(),
        }
    }
}

impl Display for DataContractEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} ({} types)",
            self.name,
            self.id_b58,
            self.document_type_names.len(),
        )
    }
}

pub(crate) struct ContractsScreenController {
    select: Option<SelectInput<DataContractEntry>>,
}

impl_builder!(ContractsScreenController);

impl ContractsScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let known_contracts_lock = app_state.known_contracts.lock().await;
        let select = if known_contracts_lock.len() > 0 {
            Some(SelectInput::new(Self::contract_entries_vec(
                known_contracts_lock.iter().map(|(k, v)| (k.clone(), v)),
            )))
        } else {
            None
        };
        ContractsScreenController { select }
    }

    fn contract_entries_vec<'a>(
        known_contracts: impl IntoIterator<Item = (String, &'a DataContract)>,
    ) -> Vec<DataContractEntry> {
        known_contracts
            .into_iter()
            .map(|(name, contract)| DataContractEntry::new(name, contract))
            .collect()
    }
}

impl ScreenController for ContractsScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(select) = &mut self.select {
            select.view(frame, area)
        } else {
            Info::new_fixed("No fetched data contracts").view(frame, area)
        }
    }

    fn name(&self) -> &'static str {
        "Contracts"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,

            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::NextScreen(FetchSystemContractScreenController::builder()),

            Event::Key(event) => {
                if let Some(select) = &mut self.select {
                    match select.on_event(*event) {
                        InputStatus::Done(DataContractEntry {
                            name,
                            document_type_names,
                            ..
                        }) => ScreenFeedback::Form(Box::new(
                            SelectDocumentTypeFormController::new(name, document_type_names),
                        )),
                        InputStatus::Redraw => ScreenFeedback::Redraw,
                        _ => ScreenFeedback::None,
                    }
                } else {
                    ScreenFeedback::None
                }
            }

            Event::Backend(
                BackendEvent::AppStateUpdated(AppStateUpdate::KnownContracts(known_contracts))
                | BackendEvent::TaskCompletedStateChange {
                    app_state_update: AppStateUpdate::KnownContracts(known_contracts),
                    ..
                },
            ) => {
                self.select = if known_contracts.len() > 0 {
                    Some(SelectInput::new(Self::contract_entries_vec(
                        known_contracts.iter().map(|(k, v)| (k.clone(), v)),
                    )))
                } else {
                    None
                };
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}
