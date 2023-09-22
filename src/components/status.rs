//! Status messages component.

use tui_realm_stdlib::Label;
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

use crate::app::Message;

#[derive(MockComponent)]
pub(crate) struct Status {
    component: Label,
}

impl Status {
    pub(crate) fn new() -> Self {
        Status {
            component: Label::default().text("No identity loaded")
        }
    }
}

impl Component<Message, NoUserEvent> for Status {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        todo!()
    }
}
