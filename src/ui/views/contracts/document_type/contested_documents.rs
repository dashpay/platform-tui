//! Contested documents screen

use dpp::{identifier::Identifier, platform_value::string_encoding::Encoding};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{documents::DocumentTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

pub struct ContestedDocumentVoteFormController {
    input: SelectInput<String>,
}

impl ContestedDocumentVoteFormController {
    pub fn new(contesting_identities: [Identifier; 2]) -> Self {
        let mut options: Vec<String> = vec!["Abstain".to_string(), "Lock".to_string()];
        for identity in contesting_identities {
            options.push(identity.to_string(Encoding::Base58));
        }
        Self {
            input: SelectInput::new(options),
        }
    }
}

impl FormController for ContestedDocumentVoteFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(vote) => FormStatus::Done {
                task: Task::Document(DocumentTask::VoteOnContestedDocument(vote)),
                block: true,
            },
            InputStatus::Exit => FormStatus::Exit,
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Vote on contested document"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        ""
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
