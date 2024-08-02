//! Usernames screen

use std::collections::BTreeMap;

use dpp::{
    platform_value::string_encoding::Encoding,
    prelude::{Identifier, Identity},
};
use itertools::Itertools;
use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use crate::{
    backend::{identities::IdentityTask, AppState, BackendEvent, Task},
    ui::screen::utils::impl_builder,
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

use super::identities::RegisterDPNSNameFormController;

const COMMAND_KEYS: [ScreenCommandKey; 6] = [
    ScreenCommandKey::new("q", "Back"),
    ScreenCommandKey::new("n", "Next identity"),
    ScreenCommandKey::new("p", "Prev identity"),
    ScreenCommandKey::new("↓", "Scroll down"),
    ScreenCommandKey::new("↑", "Scroll up"),
    ScreenCommandKey::new("r", "Register username for selected identity"),
];

pub(crate) struct DpnsUsernamesScreenController {
    identities_map: BTreeMap<Identifier, Identity>,
    identity_select: tui_realm_stdlib::List,
    identity_view: Info,
    identity_ids_vec: Vec<Identifier>,
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

        let identity_view = Info::new_scrollable(
            &known_identities_lock
                .get(&identity_ids_vec[0])
                .and_then(|identity_info| Some(as_json_string(identity_info)))
                .unwrap_or_else(String::new),
        );

        Self {
            identities_map: known_identities_lock.clone(),
            identity_select,
            identity_view,
            identity_ids_vec,
        }
    }

    fn update_identity_view(&mut self) {
        self.identity_view = Info::new_scrollable(
            &self
                .identities_map
                .get(
                    &self.identity_ids_vec
                        [self.identity_select.state().unwrap_one().unwrap_usize()],
                )
                .and_then(|v| Some(as_json_string(v)))
                .unwrap_or_else(String::new),
        );
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
            .constraints([Constraint::Max(40), Constraint::Min(1)].as_ref())
            .split(area);

        self.identity_select.view(frame, layout[0]);
        self.identity_view.view(frame, layout[1]);
    }

    fn name(&self) -> &'static str {
        "DPNS"
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
                code: Key::Char('r'),
                modifiers: KeyModifiers::NONE,
            }) => ScreenFeedback::Form(Box::new(RegisterDPNSNameFormController::new(
                self.get_selected_identity().cloned(),
            ))),

            // Identity selection keys
            Event::Key(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Down));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }
            Event::Key(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.identity_select
                    .perform(Cmd::Move(command::Direction::Up));
                self.update_identity_view();
                ScreenFeedback::Redraw
            }

            // Backend event handling
            Event::Backend(BackendEvent::TaskCompletedStateChange {
                task: Task::Identity(IdentityTask::RegisterDPNSName(..)),
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
