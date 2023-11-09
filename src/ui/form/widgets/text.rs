//! Simple text input component.

use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent},
    props::{BorderSides, Borders, Color, Style},
    tui::prelude::Rect,
    AttrValue, Attribute, Frame, MockComponent,
};

use crate::ui::form::{Input, InputStatus};

pub(crate) struct TextInput {
    input: tui_realm_stdlib::Input,
}

impl TextInput {
    pub(crate) fn new(placeholder: &'static str) -> Self {
        Self::new_init_value(placeholder, "")
    }

    pub(crate) fn new_init_value(placeholder: &'static str, value: &str) -> Self {
        // TODO tui-realm bug, wrong cursor position if no borders
        let mut input = tui_realm_stdlib::Input::default()
            .borders(Borders::default().sides(BorderSides::NONE))
            .placeholder(placeholder, Style::default().fg(Color::Gray))
            .value(value);
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        TextInput { input }
    }
}

impl Input for TextInput {
    // TODO make widget generic over parser to return b58 or hex
    type Output = String;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
        let KeyEvent { code, .. } = event;

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
