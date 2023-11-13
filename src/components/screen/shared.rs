//! Components shared across screens.

use tui_realm_stdlib::Textarea;
use tuirealm::{
    command::{Cmd, CmdResult, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Color, PropPayload, TextSpan},
    tui::prelude::Rect,
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State,
};

use crate::app::Message;

/// Textarea to represent relevant information for each screen.
pub(crate) struct Info<const SCROLLABLE: bool, const ERROR_INFO: bool> {
    component: Textarea,
}

impl<const SCROLLABLE: bool, const ERROR_INFO: bool> MockComponent
    for Info<SCROLLABLE, ERROR_INFO>
{
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.component.view(frame, area)
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        self.component.state()
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

fn str_to_spans(s: &str) -> Vec<TextSpan> {
    s.lines().map(|line| TextSpan::new(line)).collect()
}

impl Info<true, true> {
    pub(crate) fn new_error(text: &str) -> Info<true, true> {
        let component = Textarea::default()
            .highlighted_str(">")
            .foreground(Color::Red)
            .text_rows(&str_to_spans(text));
        Info { component }
    }
}

impl Info<true, false> {
    pub(crate) fn new_scrollable(text: &str) -> Info<true, false> {
        let component = Textarea::default()
            .highlighted_str(">")
            .text_rows(&str_to_spans(text));
        Info { component }
    }

    pub(crate) fn new_scrollable_text_rows(text_rows: &[TextSpan]) -> Info<true, false> {
        let component = Textarea::default()
            .highlighted_str(">")
            .text_rows(text_rows);
        Info { component }
    }
}

impl Info<false, false> {
    pub(crate) fn new_fixed(text: &str) -> Info<false, false> {
        let component = Textarea::default().text_rows(&str_to_spans(text));
        Info { component }
    }
}

/// # Events
/// In case of scrollable [Info], it reacts on up/down keys or C-p / C-n
/// shortcuts
impl<const ERROR_INFO: bool> Component<Message, NoUserEvent> for Info<true, ERROR_INFO> {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        match ev {
            Event::Keyboard(
                KeyEvent { code: Key::Up, .. }
                | KeyEvent {
                    code: Key::Char('p'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Scroll(Direction::Up));
                Some(Message::Redraw)
            }
            Event::Keyboard(
                KeyEvent {
                    code: Key::Down, ..
                }
                | KeyEvent {
                    code: Key::Char('n'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ) => {
                self.component.perform(Cmd::Scroll(Direction::Down));
                Some(Message::Redraw)
            }
            _ => None,
        }
    }
}

impl<const ERROR_INFO: bool> Component<Message, NoUserEvent> for Info<false, ERROR_INFO> {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Message> {
        None
    }
}
