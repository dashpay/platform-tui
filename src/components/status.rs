//! Status messages component.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use std::ops::Div;
use tui_realm_stdlib::Label;
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

use crate::app::state::AppState;
use crate::app::Message;

#[derive(MockComponent)]
pub(crate) struct Status {
    component: Label,
}

impl Status {
    pub(crate) fn new(state: &AppState) -> Self {
        let message = match &state.loaded_identity {
            None => "No identity loaded".to_string(),
            Some(identity) => format!("Balance {} mDash", identity.balance().div(100000000)),
        };
        Status {
            component: Label::default().text(message.as_str()),
        }
    }
}

impl Component<Message, NoUserEvent> for Status {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}
