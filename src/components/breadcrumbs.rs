//! Breadcrumbs messages component.

use tui_realm_stdlib::Label;
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

use crate::app::Message;

#[derive(MockComponent)]
pub(crate) struct Breadcrumbs {
    component: Label,
}

impl Breadcrumbs {
    pub(crate) fn new() -> Self {
        Breadcrumbs {
            component: Label::default().text("Main / "),
        }
    }
}

impl Component<Message, NoUserEvent> for Breadcrumbs {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}
