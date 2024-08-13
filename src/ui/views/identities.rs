//! UI definitions related to identities.

use std::collections::BTreeMap;

use dpp::{
    identity::accessors::IdentityGettersV0,
    platform_value::string_encoding::Encoding,
    prelude::{Identifier, Identity},
};
use itertools::Itertools;
use tui_realm_stdlib::List;
use tuirealm::{
    command::{self, Cmd},
    MockComponent,
};
use tuirealm::{
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderSides, Borders, Color, TextSpan},
    tui::{
        layout::{Constraint, Direction, Layout},
        prelude::Rect,
    },
    AttrValue, Attribute, Frame,
};

use crate::{
    backend::{as_json_string, identities::IdentityTask, AppState, BackendEvent, Task},
    ui::{
        form::{
            parsers::DefaultTextInputParser, ComposedInput, Field, FormController, FormStatus,
            Input, InputStatus, SelectInput, TextInput,
        },
        screen::{
            utils::impl_builder, widgets::info::Info, ScreenCommandKey, ScreenController,
            ScreenFeedback, ScreenToggleKey,
        },
    },
    Event,
};

use super::wallet::AddPrivateKeysFormController;

const COMMAND_KEYS: [ScreenCommandKey; 9] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("C-n", "Next identity"),
    ScreenCommandKey::new("C-p", "Prev identity"),
    ScreenCommandKey::new("↓", "Scroll identity down"),
    ScreenCommandKey::new("↑", "Scroll identity up"),
    ScreenCommandKey::new("i", "Query identity by ID"),
    ScreenCommandKey::new("t", "Transfer credits from loaded identity"),
    ScreenCommandKey::new("d", "Register DPNS name for loaded identity"),
    ScreenCommandKey::new("C-f", "Forget selected identity"),
];

pub(crate) struct IdentitiesScreenController {
    toggle_keys: [ScreenToggleKey; 1],
    identity_view: Info,
    identity_select: List,
    known_identities: BTreeMap<Identifier, Identity>,
    current_batch: Vec<Identifier>,
}

impl_builder!(IdentitiesScreenController);

impl IdentitiesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let known_identities = app_state.known_identities.lock().await;
        let known_identities_vec = known_identities
            .iter()
            .map(|(k, _)| k.clone())
            .collect_vec();
        let identity_view = Info::new_scrollable(
            &known_identities
                .first_key_value()
                .map(|(_, v)| as_json_string(v))
                .unwrap_or_else(String::new),
        );
        let mut identity_select = tui_realm_stdlib::List::default()
            .rows(
                known_identities
                    .keys()
                    .map(|v| vec![TextSpan::new(v.to_string(Encoding::Base58))])
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

        IdentitiesScreenController {
            toggle_keys: [ScreenToggleKey::new("p", "with proof")],
            identity_select,
            identity_view,
            known_identities: known_identities.clone(),
            current_batch: known_identities_vec,
        }
    }

    fn update_identity_view(&mut self) {
        self.identity_view = Info::new_scrollable(
            &self
                .current_batch
                .get(self.identity_select.state().unwrap_one().unwrap_usize())
                .map(|v| {
                    let identity_info = self
                        .known_identities
                        .get(&v)
                        .expect("expected identity to be there");
                    as_json_string(&identity_info)
                })
                .unwrap_or_else(String::new),
        );
    }

    fn get_selected_identity(&self) -> Option<&Identity> {
        let state = self.identity_select.state();
        let selected_index = state.unwrap_one().unwrap_usize();
        let identifier = self
            .current_batch
            .get(selected_index)
            .expect("expected identifier");
        self.known_identities.get(&identifier)
    }
}

impl ScreenController for IdentitiesScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(60), Constraint::Min(1)].as_ref())
            .split(area);
        self.identity_select.view(frame, layout[0]);
        self.identity_view.view(frame, layout[1]);
    }

    fn name(&self) -> &'static str {
        "Identities"
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        COMMAND_KEYS.as_ref()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        self.toggle_keys.as_ref()
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        match event {
            Event::Key(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::PreviousScreen,

            Event::Key(KeyEvent {
                code: Key::Char('i'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(GetIdentityByIdFormController::new())),

            Event::Key(KeyEvent {
                code: Key::Char('t'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(TransferCreditsFormController::new())),

            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.toggle_keys[0].toggle = !self.toggle_keys[0].toggle;
                ScreenFeedback::Redraw
            }

            Event::Key(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(RegisterDPNSNameFormController::new())),

            Event::Key(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                let selected_identifier = &self
                    .current_batch
                    .get(self.identity_select.state().unwrap_one().unwrap_usize())
                    .map(|v| {
                        let identity_info = self
                            .known_identities
                            .get(&v)
                            .expect("expected identity to be there")
                            .id();
                        as_json_string(&identity_info)
                    })
                    .unwrap_or_else(String::new);

                ScreenFeedback::Task {
                    task: Task::Identity(IdentityTask::ForgetIdentity(
                        Identifier::from_string(selected_identifier, Encoding::Base58)
                            .expect("Expected to convert string to identifier"),
                    )),
                    block: false,
                }
            }

            // Identity view keys
            Event::Key(
                key_event @ KeyEvent {
                    code: Key::Down | Key::Up,
                    modifiers: KeyModifiers::NONE,
                },
            ) => {
                self.identity_view.on_event(key_event);
                ScreenFeedback::Redraw
            }

            // Identity selection keys
            Event::Key(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Down));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }
            Event::Key(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Up));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }

            // Backend event handling
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::FetchIdentityById(..),
                execution_result,
            }) => {
                self.identity_view = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::LoadIdentityById(_)),
                execution_result: _,
                app_state_update: _,
            }) => ScreenFeedback::Form(Box::new(AddPrivateKeysFormController::new())),
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(_),
                execution_result,
                app_state_update: _,
            }) => {
                self.identity_view = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            Event::Backend(BackendEvent::TaskCompleted {
                task: Task::Identity(_),
                execution_result,
            }) => {
                self.identity_view = Info::new_from_result(execution_result);
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

pub(crate) struct GetIdentityByIdFormController {
    input: TextInput<DefaultTextInputParser<String>>, // TODO: b58 parser
}

impl GetIdentityByIdFormController {
    fn new() -> Self {
        GetIdentityByIdFormController {
            input: TextInput::new("base58 id"),
        }
    }
}

impl FormController for GetIdentityByIdFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(value) => FormStatus::Done {
                task: Task::FetchIdentityById(value, false),
                block: true,
            },
            status => status.into(),
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: tuirealm::tui::prelude::Rect) {
        self.input.view(frame, area);
    }

    fn form_name(&self) -> &'static str {
        "Get identity by ID"
    }

    fn step_name(&self) -> &'static str {
        "Base 58 ID"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

pub(crate) struct TransferCreditsFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<String>>>,
        Field<TextInput<DefaultTextInputParser<f64>>>,
    )>,
}

impl TransferCreditsFormController {
    fn new() -> Self {
        Self {
            input: ComposedInput::new((
                Field::new("Enter the recipient base58 ID", TextInput::new("Base58 ID")),
                Field::new(
                    "Enter the amount to transfer in Dash (Ex: .5)",
                    TextInput::new("Amount to transfer in Dash"),
                ),
            )),
        }
    }
}

impl FormController for TransferCreditsFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((recipient, amount)) => FormStatus::Done {
                task: Task::Identity(IdentityTask::TransferCredits(recipient, amount)),
                block: true,
            },
            status => status.into(),
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: tuirealm::tui::prelude::Rect) {
        self.input.view(frame, area);
    }

    fn form_name(&self) -> &'static str {
        "Transfer Credits"
    }

    fn step_name(&self) -> &'static str {
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        2
    }
}

pub(crate) struct RegisterDPNSNameFormController {
    input: TextInput<DefaultTextInputParser<String>>,
}

impl RegisterDPNSNameFormController {
    fn new() -> Self {
        RegisterDPNSNameFormController {
            input: TextInput::new(
                "DPNS name (example: enter \"something\" if you want \"something.dash\")",
            ),
        }
    }
}

impl FormController for RegisterDPNSNameFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(value) => FormStatus::Done {
                task: Task::Identity(IdentityTask::RegisterDPNSName(value)),
                block: true,
            },
            status => status.into(),
        }
    }

    fn step_view(&mut self, frame: &mut Frame, area: tuirealm::tui::prelude::Rect) {
        self.input.view(frame, area);
    }

    fn form_name(&self) -> &'static str {
        "Register DPNS Name"
    }

    fn step_name(&self) -> &'static str {
        "DPNS Name"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}
