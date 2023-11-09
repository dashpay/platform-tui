//! Info component definitions.

use tui_realm_stdlib::Textarea;
use tuirealm::{
    command::{Cmd, Direction},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Color, TextSpan},
    tui::prelude::Rect,
    Frame, MockComponent,
};

/// Textarea to represent relevant information for each screen.
pub(crate) struct Info {
    component: Textarea,
    scrollable: bool,
}

fn str_to_spans(s: &str) -> Vec<TextSpan> {
    s.lines().map(|line| TextSpan::new(line)).collect()
}

impl Info {
    pub(crate) fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.component.view(frame, area)
    }

    pub(crate) fn new_error(text: &str) -> Info {
        let component = Textarea::default()
            .highlighted_str(">")
            .foreground(Color::Red)
            .text_rows(&str_to_spans(text));
        Info {
            component,
            scrollable: true,
        }
    }

    pub(crate) fn new_scrollable(text: &str) -> Info {
        let component = Textarea::default()
            .highlighted_str(">")
            .text_rows(&str_to_spans(text));
        Info {
            component,
            scrollable: true,
        }
    }

    pub(crate) fn new_fixed(text: &str) -> Info {
        let component = Textarea::default().text_rows(&str_to_spans(text));
        Info {
            component,
            scrollable: false,
        }
    }

    pub(crate) fn new_from_result(result: Result<String, String>) -> Info {
        match result {
            Ok(x) => Info::new_scrollable(&x),
            Err(x) => Info::new_error(&x),
        }
    }

    /// In case of scrollable [Info], it reacts on up/down keys or C-p / C-n
    /// shortcuts
    pub(crate) fn on_event(&mut self, event: KeyEvent) -> DoRedraw {
        if !self.scrollable {
            return false;
        };

        match event {
            KeyEvent { code: Key::Up, .. }
            | KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.component.perform(Cmd::Scroll(Direction::Up));
                true
            }
            KeyEvent {
                code: Key::Down, ..
            }
            | KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.component.perform(Cmd::Scroll(Direction::Down));
                true
            }
            _ => false,
        }
    }
}

type DoRedraw = bool;
