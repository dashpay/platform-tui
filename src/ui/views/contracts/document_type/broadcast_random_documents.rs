//! Form to setup broadcasting of random documents.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{documents::DocumentTask, Task},
    ui::form::{
        parsers::DefaultTextInputParser, FormController, FormStatus, Input, InputStatus, TextInput,
    },
};

pub(super) struct BroadcastRandomDocumentsCountForm {
    input: TextInput<DefaultTextInputParser<u16>>,
    data_contract_name: String,
    document_type_name: String,
}

impl BroadcastRandomDocumentsCountForm {
    pub fn new(data_contract_name: String, document_type_name: String) -> Self {
        BroadcastRandomDocumentsCountForm {
            input: TextInput::new_init_value("Number of random documents", 1),
            data_contract_name,
            document_type_name,
        }
    }
}

impl FormController for BroadcastRandomDocumentsCountForm {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(count) => FormStatus::Done {
                task: Task::Document(DocumentTask::BroadcastRandomDocuments {
                    data_contract_name: self.data_contract_name.clone(),
                    document_type_name: self.document_type_name.clone(),
                    count,
                }),
                block: true,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Broadcast random documents"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area);
    }

    fn step_name(&self) -> &'static str {
        "Documents count"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
