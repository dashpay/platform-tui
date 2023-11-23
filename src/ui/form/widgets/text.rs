//! Simple text input component.

use std::{fmt::Display, marker::PhantomData, str::FromStr};

use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderSides, Borders, Color, Style},
    tui::prelude::Rect,
    AttrValue, Attribute, Frame, MockComponent,
};

use crate::ui::form::{Input, InputStatus};

pub(crate) struct TextInput<V> {
    input: tui_realm_stdlib::Input,
    _value_type: PhantomData<V>,
}

impl<V: Display> TextInput<V> {
    pub(crate) fn new(placeholder: &'static str) -> Self {
        Self::new_internal(placeholder, "")
    }

    pub(crate) fn new_init_value(placeholder: &'static str, value: V) -> Self {
        Self::new_internal(placeholder, &format!("{value}"))
    }

    fn new_internal(placeholder: &'static str, value: &str) -> Self {
        // TODO tui-realm bug, wrong cursor position if no borders
        let mut input = tui_realm_stdlib::Input::default()
            .borders(Borders::default().sides(BorderSides::NONE))
            .placeholder(placeholder, Style::default().fg(Color::Gray))
            .value(value);
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        TextInput {
            input,
            _value_type: PhantomData,
        }
    }
}

impl<V: FromStr> Input for TextInput<V> {
    type Output = V;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
        match event {
            KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            } => match self.input.state().unwrap_one().unwrap_string().parse() {
                Ok(value) => InputStatus::Done(value),
                Err(_) => {
                    self.input
                        .attr(Attribute::Foreground, AttrValue::Color(Color::Red));
                    InputStatus::Redraw
                }
            },
            KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            } => {
                self.input
                    .attr(Attribute::Foreground, AttrValue::Color(Color::Reset));
                self.input.perform(Cmd::Type(c));
                InputStatus::Redraw
            }
            KeyEvent {
                code: Key::Left,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Move(Direction::Left));
                InputStatus::Redraw
            }
            KeyEvent {
                code: Key::Right,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Move(Direction::Right));
                InputStatus::Redraw
            }
            KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Delete);
                self.input
                    .attr(Attribute::Foreground, AttrValue::Color(Color::Reset));
                InputStatus::Redraw
            }
            KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => InputStatus::Exit,
            _ => InputStatus::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area);
    }
}
