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

const COMMAND_KEYS: [ScreenCommandKey; 15] = [
    ScreenCommandKey::new("q", "Back to Main"),
    ScreenCommandKey::new("r", "Register new"),
    ScreenCommandKey::new("l", "Load identity with private key(s)"),
    ScreenCommandKey::new("m", "Load masternode identity"),
    ScreenCommandKey::new("s", "Set loaded"),
    ScreenCommandKey::new("t", "Transfer credits from loaded"),
    ScreenCommandKey::new("d", "Register DPNS name for loaded"),
    ScreenCommandKey::new("k", "Add key to loaded"),
    ScreenCommandKey::new("C-f", "Forget selected"),
    ScreenCommandKey::new("d", "Copy loaded ID"),
    ScreenCommandKey::new("i", "Query by ID"),
    ScreenCommandKey::new("C-n", "Next"),
    ScreenCommandKey::new("C-p", "Previous"),
    ScreenCommandKey::new("↓", "Scroll down"),
    ScreenCommandKey::new("↑", "Scroll up"),
];

#[memoize::memoize]
fn join_commands(
    identity_loaded: bool,
    identity_registration_in_progress: bool,
    identity_top_up_in_progress: bool,
) -> &'static [ScreenCommandKey] {
    let mut commands = COMMAND_KEYS.to_vec();

    if identity_loaded {
        if identity_top_up_in_progress {
            commands.push(ScreenCommandKey::new("t", "Continue top up"));
        } else {
            commands.push(ScreenCommandKey::new("t", "Top up loaded"));
        }
    } else {
        if identity_registration_in_progress {
            commands.push(ScreenCommandKey::new("r", "Continue identity registration"));
            commands.push(ScreenCommandKey::new("g", "Restart identity registration"));
        }
    }

    commands.leak()
}

pub(crate) struct IdentitiesScreenController {
    toggle_keys: [ScreenToggleKey; 1],
    identity_view: Info,
    identity_select: List,
    known_identities: BTreeMap<Identifier, Identity>,
    loaded_identity: Option<Identity>,
    current_batch: Vec<Identifier>,
    identity_registration_in_progress: bool,
    identity_top_up_in_progress: bool,
    wallet_loaded: bool,
}

impl_builder!(IdentitiesScreenController);

impl IdentitiesScreenController {
    pub(crate) async fn new(app_state: &AppState) -> Self {
        let known_identities = app_state.known_identities.lock().await;
        let known_identities_vec = known_identities
            .iter()
            .map(|(k, _)| k.clone())
            .collect_vec();

        let loaded_identity = app_state.loaded_identity.lock().await;
        let loaded_identity_id = loaded_identity.as_ref().map(|identity| identity.id());

        let identity_registration_in_progress = match *loaded_identity {
            Some(_) => false,
            None => app_state
                .identity_asset_lock_private_key_in_creation
                .lock()
                .await
                .is_some(),
        };

        let identity_top_up_in_progress = app_state
            .identity_asset_lock_private_key_in_top_up
            .lock()
            .await
            .is_some();

        let wallet_loaded = app_state.loaded_wallet.lock().await.is_some();

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
                    .map(|id| {
                        let mut text_span = TextSpan::new(id.to_string(Encoding::Base58));
                        // Check if the current id matches the loaded identity id
                        if Some(id) == loaded_identity_id.as_ref() {
                            text_span = text_span.bold(); // Make the loaded identity bold
                        }
                        vec![text_span]
                    })
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
            loaded_identity: loaded_identity.clone(),
            current_batch: known_identities_vec,
            identity_registration_in_progress,
            identity_top_up_in_progress,
            wallet_loaded,
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

    fn update_identity_list(&mut self) {
        let known_identities = self.known_identities.clone();
        let loaded_identity = self.loaded_identity.clone();
        let loaded_identity_id = loaded_identity.as_ref().map(|identity| identity.id());

        let mut new_identity_select = tui_realm_stdlib::List::default()
            .rows(
                known_identities
                    .keys()
                    .map(|id| {
                        let mut text_span = TextSpan::new(id.to_string(Encoding::Base58));
                        // Check if the current id matches the loaded identity id
                        if Some(id) == loaded_identity_id.as_ref() {
                            text_span = text_span.bold(); // Make the loaded identity bold
                        }
                        vec![text_span]
                    })
                    .collect(),
            )
            .borders(
                Borders::default()
                    .sides(BorderSides::LEFT | BorderSides::TOP | BorderSides::BOTTOM),
            )
            .highlighted_color(Color::Magenta);

        new_identity_select.attr(Attribute::Scroll, AttrValue::Flag(true));
        new_identity_select.attr(Attribute::Focus, AttrValue::Flag(true));

        self.identity_select = new_identity_select;
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
        join_commands(
            self.loaded_identity.is_some(),
            self.identity_registration_in_progress,
            self.identity_top_up_in_progress,
        )
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
            }) if self.loaded_identity.is_some() => {
                ScreenFeedback::Form(Box::new(TransferCreditsFormController::new()))
            }

            Event::Key(KeyEvent {
                code: Key::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) if self.identity_registration_in_progress => ScreenFeedback::Task {
                task: Task::Identity(IdentityTask::ClearRegistrationOfIdentityInProgress),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('m'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded => {
                ScreenFeedback::Form(Box::new(LoadMasternodeIdentityFormController::new()))
            }

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
            }) if self.loaded_identity.is_some() => ScreenFeedback::Form(Box::new(
                RegisterDPNSNameFormController::new(self.loaded_identity.clone()),
            )),

            Event::Key(KeyEvent {
                code: Key::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(LoadIdentityByIdFormController::new())),

            Event::Key(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
            }) => {
                let selected_identity = self.get_selected_identity().unwrap();
                let task = Task::Identity(IdentityTask::LoadKnownIdentity(selected_identity.id()));

                self.loaded_identity = Some(selected_identity.clone());
                self.update_identity_list();

                ScreenFeedback::Task { task, block: false }
            }

            Event::Key(KeyEvent {
                code: Key::Char('t'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded => {
                ScreenFeedback::Form(Box::new(TopUpIdentityFormController::new()))
            }

            Event::Key(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded && !self.identity_registration_in_progress => {
                ScreenFeedback::Form(Box::new(RegisterIdentityFormController::new()))
            }

            Event::Key(KeyEvent {
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) if self.wallet_loaded && self.identity_registration_in_progress => {
                ScreenFeedback::Task {
                    task: Task::Identity(IdentityTask::ContinueRegisteringIdentity),
                    block: true,
                }
            }

            Event::Key(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) if self.loaded_identity.is_some() => ScreenFeedback::Task {
                task: Task::Identity(IdentityTask::CopyIdentityId),
                block: true,
            },

            Event::Key(KeyEvent {
                code: Key::Char('f'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                if let Some(selected_identifier) = self.get_selected_identity().map(|id| id.id()) {
                    // Remove from known_identities
                    self.known_identities.remove(&selected_identifier);

                    // Find the index of the selected identifier in the current_batch and remove it
                    if let Some(index) = self
                        .current_batch
                        .iter()
                        .position(|id| *id == selected_identifier)
                    {
                        self.current_batch.remove(index);
                    }

                    // Update the identity list
                    self.update_identity_list();

                    ScreenFeedback::Task {
                        task: Task::Identity(IdentityTask::ForgetIdentity(selected_identifier)),
                        block: false,
                    }
                } else {
                    ScreenFeedback::None
                }
            }

            Event::Key(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) if self.loaded_identity.is_some() => {
                ScreenFeedback::Form(Box::new(GetIdentityByIdFormController::new()))
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
                execution_result: Ok(_),
                app_state_update: _,
            }) => ScreenFeedback::Form(Box::new(AddPrivateKeysFormController::new())),

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
    loaded_identity_option: Option<Identity>,
}

impl RegisterDPNSNameFormController {
    pub fn new(loaded_identity_option: Option<Identity>) -> Self {
        Self {
            input: TextInput::new(
                "DPNS name (example: enter \"something\" if you want \"something.dash\")",
            ),
            loaded_identity_option,
        }
    }
}

impl FormController for RegisterDPNSNameFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(value) => {
                if let Some(identity) = &self.loaded_identity_option {
                    FormStatus::Done {
                        task: Task::Identity(IdentityTask::RegisterDPNSName(
                            identity.clone(),
                            value,
                        )),
                        block: true,
                    }
                } else {
                    FormStatus::Exit
                }
            }
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

struct RegisterIdentityFormController {
    input: TextInput<DefaultTextInputParser<f64>>,
}

impl RegisterIdentityFormController {
    fn new() -> Self {
        RegisterIdentityFormController {
            input: TextInput::new("Quantity (in Dash)"),
        }
    }
}

impl FormController for RegisterIdentityFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(amount) => FormStatus::Done {
                task: Task::Identity(IdentityTask::RegisterIdentity(
                    (amount * 100000000.0) as u64,
                )),
                block: true,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity registration"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Funding amount"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

struct TopUpIdentityFormController {
    input: TextInput<DefaultTextInputParser<f64>>,
}

impl TopUpIdentityFormController {
    fn new() -> Self {
        TopUpIdentityFormController {
            input: TextInput::new("Quantity (in Dash)"),
        }
    }
}

impl FormController for TopUpIdentityFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(amount) => FormStatus::Done {
                task: Task::Identity(IdentityTask::TopUpIdentity((amount * 100000000.0) as u64)),
                block: true,
            },
            InputStatus::Redraw => FormStatus::Redraw,
            InputStatus::None => FormStatus::None,
            InputStatus::Exit => FormStatus::Exit,
        }
    }

    fn form_name(&self) -> &'static str {
        "Identity top up"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Top up amount"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

pub struct AddPrivateKeysFormController {
    private_keys: Vec<String>,
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<String>>>,
        Field<SelectInput<String>>,
    )>,
}

impl AddPrivateKeysFormController {
    pub fn new() -> Self {
        Self {
            private_keys: vec![],
            input: ComposedInput::new((
                Field::new("Private key", TextInput::new("Private key")),
                Field::new(
                    "Add another private key?",
                    SelectInput::new(vec!["No".to_string(), "Yes".to_string()]),
                ),
            )),
        }
    }
}

impl FormController for AddPrivateKeysFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((private_key, another)) => {
                self.private_keys.push(private_key);

                if another == "Yes" {
                    self.input = ComposedInput::new((
                        Field::new("Private key", TextInput::new("Private key in WIF format")),
                        Field::new(
                            "Add another private key?",
                            SelectInput::new(vec!["No".to_string(), "Yes".to_string()]),
                        ),
                    ));
                    FormStatus::Redraw
                } else {
                    FormStatus::Done {
                        task: Task::Identity(IdentityTask::AddPrivateKeys(
                            self.private_keys.clone(),
                        )),
                        block: false,
                    }
                }
            }
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Add identity with private key"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        self.input.steps_number()
    }
}

struct LoadIdentityByIdFormController {
    input: TextInput<DefaultTextInputParser<String>>,
}

impl LoadIdentityByIdFormController {
    fn new() -> Self {
        Self {
            input: TextInput::new("Identity base58 ID"),
        }
    }
}

impl FormController for LoadIdentityByIdFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done(identity_id) => FormStatus::Done {
                task: Task::Identity(IdentityTask::LoadIdentityById(identity_id)),
                block: true,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Add identity with private keys"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        "Add identity by id"
    }

    fn step_index(&self) -> u8 {
        0
    }

    fn steps_number(&self) -> u8 {
        1
    }
}

struct LoadMasternodeIdentityFormController {
    input: ComposedInput<(
        Field<TextInput<DefaultTextInputParser<String>>>,
        Field<TextInput<DefaultTextInputParser<String>>>,
    )>,
}

impl LoadMasternodeIdentityFormController {
    fn new() -> Self {
        Self {
            input: ComposedInput::new((
                Field::new(
                    "proTxHash",
                    TextInput::new("Enter Evonode proTxHash in base58 format"),
                ),
                Field::new(
                    "Private Key",
                    TextInput::new("Enter the Evonode private key in WIF format"),
                ),
            )),
        }
    }
}

impl FormController for LoadMasternodeIdentityFormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        match self.input.on_event(event) {
            InputStatus::Done((pro_tx_hash, private_key)) => FormStatus::Done {
                task: Task::Identity(IdentityTask::LoadEvonodeIdentity(pro_tx_hash, private_key)),
                block: true,
            },
            status => status.into(),
        }
    }

    fn form_name(&self) -> &'static str {
        "Load Evonode Identity"
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area)
    }

    fn step_name(&self) -> &'static str {
        self.input.step_name()
    }

    fn step_index(&self) -> u8 {
        self.input.step_index()
    }

    fn steps_number(&self) -> u8 {
        self.input.steps_number()
    }
}
