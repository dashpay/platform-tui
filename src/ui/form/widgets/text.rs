//! Simple text input component.

pub(crate) mod parsers;

use std::{fmt::Display, str::FromStr};

use tuirealm::{
    command::{self, Cmd},
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderSides, Borders, Color, Style},
    tui::{
        prelude::{Constraint, Direction, Layout, Rect},
        widgets::Paragraph,
    },
    AttrValue, Attribute, Frame, MockComponent,
};

use self::parsers::{DefaultTextInputParser, TextInputParser};
use crate::ui::form::{Input, InputStatus};

pub(crate) struct TextInput<P> {
    input: tui_realm_stdlib::Input,
    error_msg: Option<String>,
    parser: P,
}

impl<T> TextInput<DefaultTextInputParser<T>>
where
    T: FromStr + Display,
{
    pub(crate) fn new_init_value(placeholder: &'static str, value: T) -> Self {
        Self::new_str_value_with_parser(
            DefaultTextInputParser::new(),
            placeholder,
            &format!("{value}"),
        )
    }
}

impl<T> TextInput<DefaultTextInputParser<T>>
where
    T: FromStr,
{
    pub(crate) fn new(placeholder: &'static str) -> Self {
        Self::new_str_value_with_parser(DefaultTextInputParser::new(), placeholder, "")
    }
}

impl<P> TextInput<P>
where
    P: TextInputParser,
    <P as TextInputParser>::Output: Display,
{
    pub(crate) fn new_init_value_with_parser(
        parser: P,
        placeholder: &'static str,
        value: P::Output,
    ) -> Self {
        Self::new_str_value_with_parser(parser, placeholder, &format!("{value}"))
    }
}

impl<P: TextInputParser> TextInput<P> {
    pub(crate) fn new_with_parser(parser: P, placeholder: &'static str) -> Self {
        Self::new_str_value_with_parser(parser, placeholder, "")
    }

    pub(crate) fn new_str_value_with_parser(
        parser: P,
        placeholder: &'static str,
        value: &str,
    ) -> Self {
        // TODO tui-realm bug, wrong cursor position if no borders
        let mut input = tui_realm_stdlib::Input::default()
            .borders(Borders::default().sides(BorderSides::NONE))
            .placeholder(placeholder, Style::default().fg(Color::Gray))
            .value(value);
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        TextInput {
            input,
            parser,
            error_msg: None,
        }
    }

    fn set_error(&mut self, error_msg: String) {
        self.input
            .attr(Attribute::Foreground, AttrValue::Color(Color::Red));
        self.error_msg = Some(error_msg);
    }

    fn reset_error(&mut self) {
        self.input
            .attr(Attribute::Foreground, AttrValue::Color(Color::Reset));
        self.error_msg = None;
    }
}

impl<P: TextInputParser> Input for TextInput<P> {
    type Output = P::Output;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
        match event {
            KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            } => match self
                .parser
                .parse_input(&self.input.state().unwrap_one().unwrap_string())
            {
                Ok(value) => InputStatus::Done(value),
                Err(error) => {
                    self.set_error(error);
                    InputStatus::Redraw
                }
            },

            KeyEvent {
                code: Key::Left,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Move(command::Direction::Left));
                InputStatus::Redraw
            }

            KeyEvent {
                code: Key::Right,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Move(command::Direction::Right));
                InputStatus::Redraw
            }

            KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::NONE,
            } => {
                self.input.perform(Cmd::Delete);
                self.reset_error();
                InputStatus::Redraw
            }

            KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            } => InputStatus::Exit,

            KeyEvent {
                code: Key::Char(c), ..
            } => {
                self.reset_error();
                self.input.perform(Cmd::Type(c));
                InputStatus::Redraw
            }
            _ => InputStatus::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Min(2), Constraint::Min(0)].as_ref())
            .split(area);

        self.input.view(frame, layout[0]);

        if let Some(error) = &self.error_msg {
            frame.render_widget(Paragraph::new(error.as_str()), layout[1])
        }
    }
}
