//! Select-like input type definitions.

use std::{fmt::Display, marker::PhantomData};

use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderSides, Borders, Color, TextSpan},
    tui::prelude::Rect,
    AttrValue, Attribute, Frame, MockComponent,
};

use super::{Input, InputStatus};

pub(crate) struct SelectInput<V: Display + Clone> {
    input: tui_realm_stdlib::List,
    variants: Vec<V>,
    _variants_type: PhantomData<V>,
}

impl<V: Display + Clone> SelectInput<V> {
    pub(crate) fn new(variants: Vec<V>) -> Self {
        let mut input = tui_realm_stdlib::List::default()
            .rows(
                variants
                    .iter()
                    .map(|v| vec![TextSpan::new(v.to_string())])
                    .collect(),
            )
            .borders(Borders::default().sides(BorderSides::NONE))
            .highlighted_color(Color::Black);
        input.attr(Attribute::Scroll, AttrValue::Flag(true));
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        SelectInput {
            input,
            variants,
            _variants_type: PhantomData,
        }
    }
}

impl<V: Display + Clone> Input for SelectInput<V> {
    type Output = V;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
        match event {
            // Select previous line
            KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }
            | KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.input.perform(Cmd::Move(Direction::Up));
                InputStatus::Redraw
            }
            // Select next line
            KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }
            | KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.input.perform(Cmd::Move(Direction::Down));
                InputStatus::Redraw
            }
            // Confirm selections
            KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            } => InputStatus::Done(
                self.variants[self.input.state().unwrap_one().unwrap_usize()].clone(),
            ),
            _ => InputStatus::None,
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.input.view(frame, area);
    }
}
