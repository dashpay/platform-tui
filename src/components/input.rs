//! Input components.

use tuirealm::{
    command::Cmd,
    event::{Key, KeyEvent, KeyModifiers},
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{
    app::Message,
    mock_components::{key_event_to_cmd, CompletingInput, HistoryCompletionEngine},
};

#[derive(MockComponent)]
pub(crate) struct IdentityIdInput {
    component: CompletingInput<HistoryCompletionEngine>,
}

impl IdentityIdInput {
    pub(crate) fn new() -> Self {
        Self {
            component: CompletingInput::new(HistoryCompletionEngine {}),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityIdInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Message::AppClose),
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                self.component.perform(cmd);
                Some(Message::Redraw)
            }
            _ => None,
        }
    }
}
