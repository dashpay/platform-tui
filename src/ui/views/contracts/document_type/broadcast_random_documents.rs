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
}

impl BroadcastRandomDocumentsCountForm {
    pub fn new() -> Self {
        BroadcastRandomDocumentsCountForm {
            input: TextInput::new_init_value("Number of random documents", 1),
        }
    }
}

impl FormController for BroadcastRandomDocumentsCountForm {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(count) => FormStatus::Done {
                task: Task::Document(DocumentTask::BroadcastRandomDocuments {
                    data_contract: todo!(),
                    document_type: todo!(),
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
