//! Contested resources screen

use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{accessors::DocumentTypeV0Getters, DocumentType},
        DataContract,
    },
    identifier::Identifier,
    platform_value::{string_encoding::Encoding, Value},
    voting::{
        vote_choices::resource_vote_choice::ResourceVoteChoice,
        vote_polls::{
            contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll, VotePoll,
        },
    },
};
use drive_proof_verifier::types::{Contenders, ContestedResource, ContestedResources};
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{documents::DocumentTask, Task},
    ui::form::{FormController, FormStatus, Input, InputStatus, SelectInput},
};

use tuirealm::{
    command::{self, Cmd},
    event::{Key, KeyModifiers},
    props::{BorderSides, Borders, Color, TextSpan},
    tui::prelude::{Constraint, Direction, Layout},
    AttrValue, Attribute, MockComponent,
};

use crate::{
    backend::{BackendEvent, CompletedTaskPayload},
    ui::screen::{
        widgets::info::Info, ScreenCommandKey, ScreenController, ScreenFeedback, ScreenToggleKey,
    },
    Event,
};

const COMMAND_KEYS: [ScreenCommandKey; 5] = [
    ScreenCommandKey::new("q", "Back"),
    ScreenCommandKey::new("↓", "Next username"),
    ScreenCommandKey::new("↑", "Prev username"),
    ScreenCommandKey::new("s", "Status"),
    ScreenCommandKey::new("v", "Vote"),
];

pub(crate) struct ContestedResourcesScreenController {
    current_batch: ContestedResources,
    resource_select: tui_realm_stdlib::List,
    resource_view: Info,
    data_contract: DataContract,
    document_type: DocumentType,
    want_to_vote: bool,
}

impl ContestedResourcesScreenController {
    pub(crate) fn new(
        current_batch: ContestedResources,
        data_contract: DataContract,
        document_type: DocumentType,
    ) -> Self {
        let mut resource_select = tui_realm_stdlib::List::default()
            .rows(
                current_batch
                    .0
                    .iter()
                    .enumerate()
                    .map(|(_, v)| {
                        vec![TextSpan::new(match v {
                            ContestedResource::Value(value) => {
                                let value_str = value.to_string();
                                let parts: Vec<&str> = value_str.split_whitespace().collect();
                                let second_part =
                                    parts.get(1).expect("Expected a second part to the string");
                                second_part.to_string()
                            }
                        })]
                    })
                    .collect(),
            )
            .borders(
                Borders::default()
                    .sides(BorderSides::LEFT | BorderSides::TOP | BorderSides::BOTTOM),
            )
            .selected_line(0)
            .highlighted_color(Color::Magenta);
        resource_select.attr(Attribute::Scroll, AttrValue::Flag(true));
        resource_select.attr(Attribute::Focus, AttrValue::Flag(true));

        let resource_view = Info::new_fixed("Press 's' to see the voting status of this username");

        Self {
            current_batch,
            resource_select,
            resource_view,
            data_contract,
            document_type,
            want_to_vote: false,
        }
    }

    fn update_resource_view(&mut self) {
        self.resource_view = Info::new_fixed("Press 's' to see the voting status of this username");
    }

    fn get_selected_resource(&self) -> Option<&Value> {
        let state = self.resource_select.state();
        let selected_index = state.unwrap_one().unwrap_usize();
        self.current_batch
            .0
            .get(selected_index)
            .and_then(|doc| match doc {
                ContestedResource::Value(value) => Some(value),
            })
    }
}

impl ScreenController for ContestedResourcesScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(40), Constraint::Min(1)].as_ref())
            .split(area);

        self.resource_select.view(frame, layout[0]);
        self.resource_view.view(frame, layout[1]);
    }

    fn name(&self) -> &'static str {
        "Contested resources"
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
                code: Key::Char('v'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.want_to_vote = true;

                let resource = self
                    .get_selected_resource()
                    .expect("Expected to get a resource from the selection");

                let index_values = vec![Value::from("dash"), resource.clone()]; // hardcoded for dpns

                let index = self
                    .document_type
                    .find_contested_index()
                    .expect("Expected to find a contested index");

                let index_name = &index.name;

                ScreenFeedback::Task {
                    task: Task::Document(DocumentTask::QueryVoteContenders(
                        index_name.to_string(),
                        index_values,
                        self.document_type.name().to_string(),
                        self.data_contract.id(),
                    )),
                    block: true,
                }
            }
            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => {
                let resource = self
                    .get_selected_resource()
                    .expect("Expected to get a resource from the selection");

                let index_values = vec![Value::from("dash"), resource.clone()]; // hardcoded for dpns

                let index = self
                    .document_type
                    .find_contested_index()
                    .expect("Expected to find a contested index");

                let index_name = &index.name;

                ScreenFeedback::Task {
                    task: Task::Document(DocumentTask::QueryVoteContenders(
                        index_name.to_string(),
                        index_values,
                        self.document_type.name().to_string(),
                        self.data_contract.id(),
                    )),
                    block: true,
                }
            }

            // Resource selection keys
            Event::Key(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.resource_select
                    .perform(Cmd::Move(command::Direction::Down));
                self.update_resource_view();
                ScreenFeedback::Redraw
            }
            Event::Key(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.resource_select
                    .perform(Cmd::Move(command::Direction::Up));
                self.update_resource_view();
                ScreenFeedback::Redraw
            }

            // Backend events handling
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Document(DocumentTask::QueryVoteContenders(_, _, _, _)),
                execution_result,
            }) => match execution_result {
                Ok(CompletedTaskPayload::ContestedResourceContenders(vote_poll, contenders)) => {
                    if self.want_to_vote {
                        ScreenFeedback::Form(Box::new(ContestedResourceVoteFormController::new(
                            vote_poll.clone(),
                            contenders.clone(),
                        )) as Box<dyn FormController>)
                    } else {
                        let mut options: Vec<String> = vec![
                            format!(
                                "{} - Abstain",
                                contenders.abstain_vote_tally.unwrap_or_default()
                            ),
                            format!("{} - Lock", contenders.lock_vote_tally.unwrap_or_default()),
                        ];
                        for contender in contenders.contenders.clone() {
                            let identity_id = contender.0;
                            options.push(format!(
                                "{} - {}",
                                contender.1.vote_tally().unwrap_or_default(),
                                identity_id.to_string(Encoding::Base58),
                            ));
                        }

                        self.resource_view = Info::new_fixed(&options.join("\n"));

                        ScreenFeedback::Redraw
                    }
                }
                Err(_) => {
                    self.resource_view = Info::new_from_result(execution_result);
                    ScreenFeedback::Redraw
                }
                _ => ScreenFeedback::None,
            },

            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Document(DocumentTask::VoteOnContestedResource(_, _)),
                execution_result,
            }) => {
                self.want_to_vote = false;
                self.resource_view = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }

            _ => ScreenFeedback::None,
        }
    }
}

pub struct ContestedResourceVoteFormController {
    input: SelectInput<String>,
    vote_poll: ContestedDocumentResourceVotePoll,
}

impl ContestedResourceVoteFormController {
    pub fn new(vote_poll: ContestedDocumentResourceVotePoll, contenders: Contenders) -> Self {
        let mut options: Vec<String> = vec![
            format!(
                "{} - Abstain",
                contenders.abstain_vote_tally.unwrap_or_default()
            ),
            format!("{} - Lock", contenders.lock_vote_tally.unwrap_or_default()),
        ];
        for contender in contenders.contenders {
            let identity_id = contender.0;
            options.push(format!(
                "{} - {}",
                contender.1.vote_tally().unwrap_or_default(),
                identity_id.to_string(Encoding::Base58),
            ));
        }
        Self {
            input: SelectInput::new(options),
            vote_poll,
        }
    }
}

impl FormController for ContestedResourceVoteFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(vote_string) => {
                tracing::info!("{vote_string}");
                let parsed_vote_string = vote_string.split_whitespace().last().unwrap_or("");
                tracing::info!("{parsed_vote_string}");
                let vote = match parsed_vote_string {
                    "Abstain" => ResourceVoteChoice::Abstain,
                    "Lock" => ResourceVoteChoice::Lock,
                    _ => ResourceVoteChoice::TowardsIdentity(
                        Identifier::from_string(&parsed_vote_string, Encoding::Base58)
                            .expect("Expected to convert String to Identifier"),
                    ),
                };
                FormStatus::Done {
                    task: Task::Document(DocumentTask::VoteOnContestedResource(
                        VotePoll::ContestedDocumentResourceVotePoll(self.vote_poll.clone()),
                        vote,
                    )),
                    block: true,
                }
            }
            InputStatus::Exit => FormStatus::Exit,
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Vote on contested resource"
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
