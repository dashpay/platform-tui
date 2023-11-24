//! UI defenitions for selected data contract.

use dpp::data_contract::{
    accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
};
use futures::FutureExt;
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

const COMMANDS: [ScreenCommandKey; 1] = [ScreenCommandKey::new("q", "Back to Contracts")];

pub(super) struct DocumentTypeScreenController {
    info: Info,
}

impl DocumentTypeScreenController {
    pub(super) async fn new(
        contract_name: String,
        document_type_name: String,
        app_state: &AppState,
    ) -> Self {
        let known_contracts_lock = app_state.known_contracts.lock().await;
        let document_type = known_contracts_lock
            .get(&contract_name)
            .map(|contract| contract.document_types().get(&document_type_name))
            .flatten();
        let document_type_str = document_type
            .map(|dt| as_toml(dt.properties()))
            .unwrap_or_else(|| "unknown document type".to_owned());
        let info = Info::new_scrollable(&document_type_str);

        DocumentTypeScreenController { info }
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
