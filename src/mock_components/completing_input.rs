//! Text input that is capable of providing input completions.

mod completions;

use std::ops::Deref;

pub(crate) use completions::*;
use tui_realm_stdlib::{Input, List};
use tuirealm::{
    command::{self, Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Color, InputType, Style, TextSpan},
    tui::prelude::{Constraint, Direction, Layout, Rect},
    AttrValue, Attribute, Frame, MockComponent, State, StateValue,
};

// Helper function to translate key events to widget commands
pub(crate) fn key_event_to_cmd(key: KeyEvent) -> Cmd {
    match key {
        KeyEvent {
            code: Key::Backspace,
            ..
        } => Cmd::Delete,
        KeyEvent {
            code: Key::Enter, ..
        } => Cmd::Submit,

        // Commands to navigate input
        KeyEvent {
            code: Key::Left, ..
        } => Cmd::Move(command::Direction::Left),
        KeyEvent {
            code: Key::Right, ..
        } => Cmd::Move(command::Direction::Right),
        KeyEvent {
            code: Key::Char('b'),
            modifiers: KeyModifiers::CONTROL,
        } => Cmd::Move(command::Direction::Left),
        KeyEvent {
            code: Key::Char('f'),
            modifiers: KeyModifiers::CONTROL,
        } => Cmd::Move(command::Direction::Right),

        // Commands to navigate completions list
        KeyEvent { code: Key::Up, .. } => Cmd::Move(command::Direction::Up),
        KeyEvent {
            code: Key::Down, ..
        } => Cmd::Move(command::Direction::Down),
        KeyEvent {
            code: Key::Char('n'),
            modifiers: KeyModifiers::CONTROL,
        } => Cmd::Move(command::Direction::Down),
        KeyEvent {
            code: Key::Char('p'),
            modifiers: KeyModifiers::CONTROL,
        } => Cmd::Move(command::Direction::Up),

        KeyEvent {
            code: Key::Char('q'),
            modifiers: KeyModifiers::CONTROL,
        } => Cmd::Cancel,
        KeyEvent {
            code: Key::Char(c), ..
        } => Cmd::Type(c),
        _ => Cmd::None,
    }
}

pub(crate) struct CompletingInput<C> {
    _completion_engine: C,
    input: Input,
    variants: List,
    choosing_completion: bool,
}

impl<C: CompletionEngine> CompletingInput<C> {
    pub(crate) fn new(completion_engine: C, placeholder: &'static str) -> Self {
        let mut input = Input::default()
            .placeholder(placeholder, Style::default().fg(Color::Gray))
            .input_type(InputType::Text);
        input.attr(Attribute::Focus, AttrValue::Flag(true));

        let completions = completion_engine
            .get_completions_list("")
            .map(|c| vec![TextSpan::new(c.deref())])
            .collect();

        let variants = List::default()
            .rows(completions)
            .highlighted_color(Color::LightYellow);

        Self {
            _completion_engine: completion_engine,
            input,
            variants,
            choosing_completion: false,
        }
    }

    fn cancel_completion(&mut self) -> CmdResult {
        self.variants
            .attr(Attribute::Scroll, AttrValue::Flag(false));
        self.variants.attr(Attribute::Focus, AttrValue::Flag(false));
        self.choosing_completion = false;
        CmdResult::None
    }
}

impl<C: CompletionEngine> MockComponent for CompletingInput<C> {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(3), Constraint::Min(4)].as_ref())
            .split(area);
        self.input.view(frame, layout[0]);
        self.variants.view(frame, layout[1]);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        if self.choosing_completion {
            match cmd {
                move_input @ Cmd::Move(command::Direction::Up | command::Direction::Down) => {
                    self.variants.perform(move_input);
                    CmdResult::None
                }
                Cmd::Submit => {
                    match self.variants.state() {
                        State::One(StateValue::Usize(idx)) => {
                            if let AttrValue::Table(table) = self
                                .variants
                                .query(Attribute::Content)
                                .expect("list always has a content table")
                            {
                                self.input.attr(
                                    Attribute::Value,
                                    AttrValue::String(table[idx][0].content.clone()),
                                )
                            }
                        }
                        _ => (),
                    };
                    self.cancel_completion()
                }
                Cmd::Cancel => self.cancel_completion(),
                _ => CmdResult::None,
            }
        } else {
            match cmd {
                char_input @ Cmd::Type(_) => self.input.perform(char_input),
                Cmd::Delete => self.input.perform(Cmd::Delete),
                move_input @ Cmd::Move(command::Direction::Left | command::Direction::Right) => {
                    self.input.perform(move_input)
                }
                Cmd::Move(command::Direction::Up | command::Direction::Down) => {
                    self.choosing_completion = true;
                    self.variants.attr(Attribute::Scroll, AttrValue::Flag(true));
                    self.variants.attr(Attribute::Focus, AttrValue::Flag(true));
                    CmdResult::None
                }
                Cmd::Submit => CmdResult::Submit(self.input.state()),
                Cmd::Cancel => CmdResult::Submit(State::None),
                _ => CmdResult::None,
            }
        }
    }
}
