//! UI defenitions for selected data contract.

use std::ops::Deref;
use dash_platform_sdk::platform::DriveQuery;
use dpp::data_contract::{
    accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
};
use dpp::data_contract::document_type::DocumentType;
use dpp::identifier::Identifier;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::DataContract;
use futures::{FutureExt, StreamExt};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    tui::prelude::Rect,
    Frame,
};

use super::ContractsScreenController;
use crate::{
    backend::{as_toml, AppState},
    ui::{
        form::{FormController, FormStatus, Input, InputStatus, SelectInput},
        screen::{
            widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback,
            ScreenToggleKey,
        },
    },
    Event,
};
use crate::backend::documents::DocumentTask;
use crate::backend::Task;
use crate::ui::form::TextInput;

pub(super) struct SelectDocumentTypeFormController {
    input: SelectInput<String>,
    contract_name: String,
}

impl SelectDocumentTypeFormController {
    pub(super) fn new(contract_name: String, document_type_names: Vec<String>) -> Self {
        SelectDocumentTypeFormController {
            input: SelectInput::new(document_type_names),
            contract_name,
        }
    }
}

impl FormController for SelectDocumentTypeFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(document_type_name) => {
                let contract_name = self.contract_name.clone();
                let document_type_name = document_type_name.clone();

                FormStatus::NextScreen(Box::new(|app_state| {
                    async {
                        Box::new(
                            DocumentTypeScreenController::new(
                                app_state.loaded_identity.lock().await.as_ref().map(|identity| identity.id()),
                                contract_name,
                                document_type_name,
                                app_state,
                            )
                            .await,
                        ) as Box<dyn ScreenController>
                    }
                    .boxed()
                }))
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Examine document type of the data contract"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Document type"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

const COMMANDS: [ScreenCommandKey; 4] = [
    ScreenCommandKey::new("q", "Back to Contracts"),
    ScreenCommandKey::new("f", "Query"),
    ScreenCommandKey::new("o", "Query ours"),
    ScreenCommandKey::new("r", "Broadcast Random Document"),
];

pub(super) struct DocumentTypeScreenController {
    identity_identifier: Option<Identifier>,
    data_contract: DataContract,
    document_type: DocumentType,
    info: Info,
}

impl DocumentTypeScreenController {
    pub(super) async fn new(
        identity_identifier: Option<Identifier>,
        contract_name: String,
        document_type_name: String,
        app_state: &AppState,
    ) -> Self {
        let known_contracts_lock = app_state.known_contracts.lock().await;
        let data_contract = known_contracts_lock
            .get(&contract_name).expect("expected a contract").clone();
        let document_type = data_contract
            .document_type_for_name(&document_type_name)
            .expect("expected a document type").to_owned_document_type();
        let document_type_str = as_toml(document_type.properties());
        let info = Info::new_scrollable(&document_type_str);

        DocumentTypeScreenController { identity_identifier, data_contract, document_type, info }
    }
}

impl ScreenController for DocumentTypeScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.info.view(frame, area)
    }

    fn name(&self) -> &'static str {
        "Document type"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        &COMMANDS
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        &[]
    }

    fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen(ContractsScreenController::builder()),

            Event::Key(KeyEvent {
                           code: Key::Char('f'),
                           modifiers: KeyModifiers::NONE,
                       }) => {
                ScreenFeedback::Form(Box::new(QueryDocumentTypeFormController::new(self.data_contract.clone(), self.document_type.clone(), None)))
            }

            Event::Key(KeyEvent {
                           code: Key::Char('o'),
                           modifiers: KeyModifiers::NONE,
                       }) => {
                ScreenFeedback::Form(Box::new(QueryDocumentTypeFormController::new(self.data_contract.clone(), self.document_type.clone(), self.identity_identifier.clone())))
            }

            Event::Key(k) => {
                if self.info.on_event(k) {
                    ScreenFeedback::Redraw
                } else {
                    ScreenFeedback::None
                }
            }

            _ => ScreenFeedback::None,
        }
    }
}


struct QueryDocumentTypeFormController {
    data_contract: DataContract,
    document_type: DocumentType,
    input: TextInput<String>, // TODO: provide parser to always have a typesafe valid output
}

impl QueryDocumentTypeFormController {
    fn new(data_contract: DataContract, document_type: DocumentType, ours_query: Option<Identifier>) -> Self {
        let ours_query_part = if let Some(ours_identifier) = ours_query {
            format!("where ownerId = {} ", ours_identifier.to_string(Encoding::Base58))
        } else {
            String::default()
        };
        let query = format!("Select * from {} {}", document_type.name(), ours_query_part);
        QueryDocumentTypeFormController {
            data_contract,
            document_type,
            input: TextInput::new_init_value("Document Query", query),
        }
    }
}

impl FormController for QueryDocumentTypeFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(query) => {
                let drive_query_result = DriveQuery::from_sql_expr(query.as_str(), &self.data_contract, None);

                match drive_query_result {
                    Ok(drive_query) => {
                        FormStatus::Done {
                            task: Task::Document(DocumentTask::QueryDocuments(drive_query.into())),
                            block: false,
                        }
                    }
                    Err(e) => {
                        FormStatus::Error(e.to_string())
                    }
                }
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Get Documents by Query"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Query"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
