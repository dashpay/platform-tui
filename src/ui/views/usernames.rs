//! Usernames screen

use futures::FutureExt;
use std::collections::BTreeMap;

use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    data_contracts::dpns_contract,
    document::DocumentV0Getters,
    identity::accessors::IdentityGettersV0,
    platform_value::string_encoding::Encoding,
    prelude::{DataContract, Identifier, Identity},
};
use drive_proof_verifier::types::ContestedResource;
use itertools::Itertools;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame, StateValue};

use crate::{
    backend::{
        documents::DocumentTask, identities::IdentityTask, AppState, BackendEvent,
        CompletedTaskPayload, ContractTask, Task,
    },
    ui::{
        form::parsers::{DocumentQueryTextInputParser, TextInputParser},
        screen::utils::impl_builder,
    },
};

use tuirealm::{
    command::{self, Cmd},
    event::{Key, KeyModifiers},
    props::{BorderSides, Borders, Color, TextSpan},
    tui::prelude::{Constraint, Direction, Layout},
    AttrValue, Attribute, MockComponent,
};

use crate::{
    backend::as_json_string,
    ui::screen::{
        widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

use super::{
    contracts::document_type::contested_resources::ContestedResourcesScreenController,
    identities::RegisterDPNSNameFormController,
};

const DPNS_UNKNOWN_COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back"),
    ScreenCommandKey::new("f", "Fetch DPNS contract"),
];

const DPNS_KNOWN_COMMAND_KEYS: [ScreenCommandKey; 8] = [
    ScreenCommandKey::new("q", "Back"),
    ScreenCommandKey::new("n", "Next identity"),
    ScreenCommandKey::new("p", "Prev identity"),
    ScreenCommandKey::new("↓", "Scroll down"),
    ScreenCommandKey::new("↑", "Scroll up"),
    ScreenCommandKey::new("r", "Register username for selected identity"),
    ScreenCommandKey::new("g", "Query names for selected identity"),
    ScreenCommandKey::new("v", "Voting screen"),
];

const DPNS_KNOWN_NO_IDENTITIES_COMMAND_KEYS: [ScreenCommandKey; 2] = [
    ScreenCommandKey::new("q", "Back"),
    ScreenCommandKey::new("v", "Voting screen"),
];

pub(crate) struct DpnsUsernamesScreenController {
    identities_map: BTreeMap<Identifier, Identity>,
    identities_names_map: BTreeMap<Identifier, Vec<String>>,
    identity_select: tui_realm_stdlib::List,
    identity_view: Info,
    identity_ids_vec: Vec<Identifier>,
    dpns_contract: Option<DataContract>,
}

impl_builder!(DpnsUsernamesScreenController);

impl DpnsUsernamesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let known_identities_lock = app_state.known_identities.lock().await;
        let identity_ids_vec = known_identities_lock.iter().map(|(k, _)| *k).collect_vec();
        let mut identity_select = tui_realm_stdlib::List::default()
            .rows(
                identity_ids_vec
                    .iter()
                    .map(|identifier| vec![TextSpan::new(identifier.to_string(Encoding::Base58))])
                    .collect(),
            )
            .borders(
                Borders::default()
                    .sides(BorderSides::LEFT | BorderSides::TOP | BorderSides::BOTTOM),
            )
            .selected_line(0)
            .highlighted_color(Color::Magenta);
        identity_select.attr(Attribute::Scroll, AttrValue::Flag(true));
        identity_select.attr(Attribute::Focus, AttrValue::Flag(true));

        let known_contracts_lock = app_state.known_contracts.lock().await;
        let maybe_dpns_contract = match known_contracts_lock.get(
            &Identifier::from_bytes(&dpns_contract::ID_BYTES)
                .unwrap()
                .to_string(Encoding::Base58),
        ) {
            Some(contract) => Some(contract),
            None => known_contracts_lock.get(&String::from("DPNS")),
        };

        let identity_view = if maybe_dpns_contract.is_some() {
            if let Some(first_identity_id) = identity_ids_vec.get(0) {
                Info::new_scrollable(
                    &known_identities_lock
                        .get(first_identity_id)
                        .and_then(|identity_info| Some(as_json_string(identity_info)))
                        .unwrap_or_else(String::new),
                )
            } else {
                Info::new_scrollable(&String::from("No known identities"))
            }
        } else {
            Info::new_fixed("DPNS contract not known yet. Please press 'f' to fetch it.")
        };

        let identities_names_map_lock = app_state.known_identities_names.lock().await;

        Self {
            identities_map: known_identities_lock.clone(),
            identities_names_map: identities_names_map_lock.clone(),
            identity_select,
            identity_view,
            identity_ids_vec,
            dpns_contract: maybe_dpns_contract.cloned(),
        }
    }

    fn update_identity_view(&mut self) {
        match self.identity_select.state().unwrap_one() {
            StateValue::Usize(selected_index) => {
                if let Some(identity_id) = self.identity_ids_vec.get(selected_index) {
                    if let Some(identity) = self.identities_map.get(identity_id) {
                        self.identity_view = Info::new_scrollable(&as_json_string(identity));
                    }
                } else {
                    self.identity_view = Info::new_scrollable(&String::from("No known identities"));
                }
            }
            _ => {
                self.identity_view = Info::new_scrollable(&String::new());
            }
        }
    }

    fn get_selected_identity(&self) -> Option<&Identity> {
        let selected_identity_string = &self.identity_ids_vec
            [self.identity_select.state().unwrap_one().unwrap_usize()]
        .to_string(Encoding::Base58);
        self.identities_map
            .get(&Identifier::from_string(&selected_identity_string, Encoding::Base58).unwrap())
    }
}

impl ScreenController for DpnsUsernamesScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(48), Constraint::Min(1)].as_ref())
            .split(area);

        self.identity_select.view(frame, layout[0]);
        self.identity_view.view(frame, layout[1]);
    }

    fn name(&self) -> &'static str {
        "DPNS"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        if self.dpns_contract.is_some() && self.identity_ids_vec.len() > 0 {
            DPNS_KNOWN_COMMAND_KEYS.as_ref()
        } else if self.dpns_contract.is_none() {
            DPNS_UNKNOWN_COMMAND_KEYS.as_ref()
        } else {
            DPNS_KNOWN_NO_IDENTITIES_COMMAND_KEYS.as_ref()
        }
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
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) if self.identity_ids_vec.len() > 0 => ScreenFeedback::Form(Box::new(
                RegisterDPNSNameFormController::new(self.get_selected_identity().cloned()),
            )),
            Event::Key(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) if self.identity_ids_vec.len() > 0 => {
                let ours_query_part = format!(
                    "where `records.identity` = '{}' ", // hardcoded for dpns. $ownerId only works if its indexed.
                    self.get_selected_identity()
                        .unwrap()
                        .id()
                        .to_string(Encoding::Base58)
                );
                let query = format!(
                    "Select * from {} {}",
                    self.dpns_contract
                        .clone()
                        .unwrap()
                        .document_type_cloned_for_name("domain")
                        .unwrap()
                        .name(),
                    ours_query_part
                );
                let parser = DocumentQueryTextInputParser::new(self.dpns_contract.clone().unwrap());
                match parser.parse_input(&query) {
                    Ok(document_query) => ScreenFeedback::Task {
                        task: Task::Document(DocumentTask::QueryDocumentsAndContestedResources {
                            document_query,
                            data_contract: self.dpns_contract.clone().unwrap(),
                            document_type: self
                                .dpns_contract
                                .clone()
                                .unwrap()
                                .document_type_cloned_for_name("domain")
                                .unwrap(),
                        }),
                        block: true,
                    },
                    Err(e) => {
                        // Handle the error appropriately, for example, by logging it or showing a message
                        self.identity_view =
                            Info::new_error(&format!("Failed to parse query properly: {}", e));
                        ScreenFeedback::Redraw
                    }
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if self.dpns_contract.is_some() {
                    ScreenFeedback::Task {
                        task: Task::Document(DocumentTask::QueryContestedResources(
                            self.dpns_contract
                                .clone()
                                .expect("Expected dpns contract to be loaded")
                                .clone(),
                            self.dpns_contract
                                .clone()
                                .expect("Expected dpns contract to be loaded")
                                .document_type_cloned_for_name("domain")
                                .expect("Expected domain document type to be in dpns contract")
                                .clone(),
                        )),
                        block: true,
                    }
                } else {
                    self.identity_view = Info::new_fixed(
                        "DPNS contract not known yet. Please press 'f' to fetch it.",
                    );
                    ScreenFeedback::Redraw
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Task {
                task: Task::Contract(ContractTask::FetchDPNSContract),
                block: true,
            },

            // Identity selection keys
            Event::Key(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Down));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }
            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Up));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }

            // Scrolling
            Event::Key(
                key_event @ KeyEvent {
                    code: Key::Down | Key::Up,
                    modifiers: KeyModifiers::NONE,
                },
            ) => {
                self.identity_view.on_event(key_event);
                ScreenFeedback::Redraw
            }

            // Backend event handling
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::RegisterDPNSName(..)),
                execution_result,
                app_state_update,
            }) => {
                self.identity_view = Info::new_from_result(execution_result);
                let registered_name = match app_state_update {
                    crate::backend::AppStateUpdate::DPNSNameRegistered(name) => name,
                    crate::backend::AppStateUpdate::DPNSNameRegistrationFailed(e) => {
                        self.identity_view = Info::new_error(&format!(
                            "Failed to register DPNS name: {}",
                            e.to_string()
                        ));
                        return ScreenFeedback::Redraw;
                    }
                    _ => todo!(),
                };
                self.identities_names_map
                    .entry(self.get_selected_identity().unwrap().id())
                    .or_insert_with(Vec::new)
                    .push(registered_name.to_string());

                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(_),
                execution_result,
            }) => {
                self.identity_view = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Document(DocumentTask::QueryDocumentsAndContestedResources { .. }),
                execution_result:
                    Ok(CompletedTaskPayload::DocumentsAndContestedResources(documents, resources)),
            }) => {
                let owned_names_vec: Vec<_> = documents
                    .iter()
                    .filter_map(|document| document.1.clone().unwrap().get("label").cloned())
                    .collect_vec();

                let selected_identity = self
                    .get_selected_identity()
                    .expect("Expected to have a selected identity");

                let selected_identity_names = self
                    .identities_names_map
                    .entry(selected_identity.id())
                    .or_insert_with(Vec::new);

                let mut contested_names_vec = resources
                    .0
                    .iter()
                    .map(|v| match v {
                        ContestedResource::Value(value) => value
                            .to_string()
                            .split_whitespace()
                            .nth(1)
                            .unwrap()
                            .to_string(),
                    })
                    .collect_vec();

                contested_names_vec.retain(|name| selected_identity_names.contains(name));

                self.identity_view = Info::new_scrollable(&format!(
                    "Owned names: {}\n\nContested names: {}",
                    as_json_string(&owned_names_vec),
                    as_json_string(&contested_names_vec)
                ));
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Document(DocumentTask::QueryDocuments(_)),
                execution_result: Err(e),
            }) => {
                self.identity_view = Info::new_error(&format!("Failed to get names: {}", e));
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Document(DocumentTask::QueryContestedResources(_, _)),
                execution_result: Ok(CompletedTaskPayload::ContestedResources(resources)),
            }) => {
                if self.dpns_contract.is_some() {
                    let resources = resources.clone();
                    let data_contract = self
                        .dpns_contract
                        .clone()
                        .expect("Expected dpns contract to be loaded")
                        .clone();
                    let document_type = self
                        .dpns_contract
                        .clone()
                        .expect("Expected dpns contract to be loaded")
                        .document_type_cloned_for_name("domain")
                        .expect("Expected domain document type to be in dpns contract");
                    ScreenFeedback::NextScreen(Box::new(move |_| {
                        async move {
                            Box::new(ContestedResourcesScreenController::new(
                                resources,
                                data_contract,
                                document_type,
                            )) as Box<dyn ScreenController>
                        }
                        .boxed()
                    }))
                } else {
                    self.identity_view = Info::new_fixed(
                        "DPNS contract not known yet. Please press 'f' to fetch it.",
                    );
                    ScreenFeedback::Redraw
                }
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Contract(ContractTask::FetchDPNSContract),
                execution_result,
                app_state_update,
            }) => {
                if execution_result.is_ok() {
                    match app_state_update {
                        crate::backend::AppStateUpdate::KnownContracts(contracts_lock) => {
                            let dpns_contract = match contracts_lock.get(
                                &Identifier::from_bytes(&dpns_contract::ID_BYTES)
                                    .unwrap()
                                    .to_string(Encoding::Base58),
                            ) {
                                Some(contract) => Some(contract),
                                None => contracts_lock.get(&String::from("DPNS")),
                            };
                            self.dpns_contract = dpns_contract.cloned();
                        }
                        _ => todo!(),
                    }
                    self.update_identity_view();
                }
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Contract(ContractTask::FetchWithdrawalsContract),
                execution_result,
                app_state_update,
            }) => {
                if execution_result.is_ok() {
                    match app_state_update {
                        crate::backend::AppStateUpdate::KnownContracts(contracts_lock) => {
                            let dpns_contract = match contracts_lock.get(
                                &Identifier::from_bytes(&dpns_contract::ID_BYTES)
                                    .unwrap()
                                    .to_string(Encoding::Base58),
                            ) {
                                Some(contract) => Some(contract),
                                None => contracts_lock.get(&String::from("Withdrawals")),
                            };
                            self.dpns_contract = dpns_contract.cloned();
                        }
                        _ => todo!(),
                    }
                    self.update_identity_view();
                }
                ScreenFeedback::Redraw
            }

            _ => ScreenFeedback::None,
        }
    }
}
