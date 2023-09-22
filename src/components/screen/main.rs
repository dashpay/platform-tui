//! Main screen module, also known as a welcome screen.

use tui_realm_stdlib::Paragraph;
use tuirealm::{props::TextSpan, Component, Event, MockComponent, NoUserEvent};

use crate::app::Message;

#[derive(MockComponent)]
pub(crate) struct MainScreen {
    component: Paragraph,
}

impl MainScreen {
    pub(crate) fn new() -> Self {
        MainScreen {
            component: Paragraph::default().text(
                [TextSpan::new(
                    "Welcome to Platform TUI!
Use keys listed in \"Commands\" section below to switch screens and/or toggle flags.
Some of them require signature and are disabled until an identity key is loaded.",
                )]
                .as_ref(),
            ),
        }
    }
}

impl Component<Message, NoUserEvent> for MainScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        todo!()
    }
}
