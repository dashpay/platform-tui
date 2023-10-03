//! Input components.

use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
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
        let mut completions = HistoryCompletionEngine::default();
        completions.add_history_item("Test1".to_owned());
        completions.add_history_item("Test2".to_owned());

        Self {
            component: CompletingInput::new(completions, "base58 Identity ID"),
        }
    }
}

impl Component<Message, NoUserEvent> for IdentityIdInput {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(key_event) => {
                let cmd = key_event_to_cmd(key_event);
                match self.component.perform(cmd) {
                    CmdResult::Changed(_) => Some(Message::Redraw),
                    CmdResult::Submit(State::One(StateValue::String(s))) => {
                        Some(Message::FetchIdentityById(s))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
