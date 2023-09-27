//! Text input that is capable of providing input completions.

mod completions;

use tui_realm_stdlib::{Input, List};
use tuirealm::{
    command::{self, Cmd, CmdResult},
    event::{Key, KeyEvent},
    props::{Alignment, Color, InputType, Style},
    tui::prelude::{Constraint, Direction, Layout, Rect},
    AttrValue, Attribute, Frame, MockComponent, Props, State,
};

pub(crate) use completions::*;

pub(crate) fn key_event_to_cmd(key: KeyEvent) -> Cmd {
    let code = key.code;
    match code {
        Key::Backspace => Cmd::Delete,
        Key::Enter => Cmd::Submit,
        Key::Left => Cmd::Move(command::Direction::Left),
        Key::Right => Cmd::Move(command::Direction::Right),
        Key::Up => Cmd::Move(command::Direction::Up),
        Key::Down => Cmd::Move(command::Direction::Down),
        Key::Char(c) => Cmd::Type(c),
        _ => Cmd::None,
    }
}

pub(crate) struct CompletingInput<C> {
    completion_engine: C,
    input: Input,
    variants: List,
}

impl<C> CompletingInput<C> {
    pub(crate) fn new(completion_engine: C) -> Self {
        let mut input = Input::default()
            .placeholder("base58 ID", Style::default().fg(Color::Gray))
            .input_type(InputType::Text);
        input.attr(Attribute::Focus, AttrValue::Flag(true));
        Self {
            completion_engine,
            input,
            variants: Default::default(),
        }
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

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            char_input @ Cmd::Type(_) => self.input.perform(char_input),
            Cmd::Delete => self.input.perform(Cmd::Delete),
            move_input @ Cmd::Move(_) => self.input.perform(move_input),
            Cmd::Scroll(_) => todo!(),
            Cmd::GoTo(_) => todo!(),
            Cmd::Submit => todo!(),
            Cmd::Cancel => todo!(),
            Cmd::Toggle => todo!(),
            Cmd::Change => todo!(),
            Cmd::Tick => todo!(),
            Cmd::Custom(_) => todo!(),
            Cmd::None => todo!(),
        }
    }
}
