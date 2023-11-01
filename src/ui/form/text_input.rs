//! Simple text input component.

use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent},
    props::{Color, Style},
    tui::prelude::Rect,
    AttrValue, Attribute, Frame, MockComponent,
};

use super::{Input, InputStatus};

pub(crate) struct TextInput {
    input: tui_realm_stdlib::Input,
}

impl TextInput {
    pub(crate) fn new(placeholder: &'static str) -> Self {
        let mut input = tui_realm_stdlib::Input::default()
            .placeholder(placeholder, Style::default().fg(Color::Gray));
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        TextInput { input }
    }
}

impl Input for TextInput {
    type Output = String;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
        let KeyEvent { code, modifiers } = event;

        match code {
            Key::Enter => InputStatus::Done(self.input.state().unwrap_one().unwrap_string()),
            Key::Char(c) => {
                self.input.perform(Cmd::Type(c));
                InputStatus::Redraw
            }
            Key::Left => {
                self.input.perform(Cmd::Move(Direction::Left));
                InputStatus::Redraw
            }
            Key::Right => {
                self.input.perform(Cmd::Move(Direction::Right));
                InputStatus::Redraw
            }
            Key::Backspace => {
                self.input.perform(Cmd::Delete);
                InputStatus::Redraw
            }
            _ => InputStatus::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area);
    }
}
