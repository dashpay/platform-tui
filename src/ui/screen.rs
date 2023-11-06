//! Screen module.

mod command_pallet;
mod info;

use std::ops::{Deref, DerefMut};

use info::Info;
use tuirealm::{
    event::KeyEvent,
    tui::prelude::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::{form::FormController, BackendEvent, Event};

/// Screen is the unit of navigation and representation in the TUI.
/// It consists of two blocks:
/// 1. an info block that represents help, optionally scrollable data or error,
/// 2. a command pallet: a table of keystokes for commands and toggles acts like
///    a help, also indicates toggles' states
///
/// Because all the screens are the same thing to draw it's one generic type,
/// however, they're different about what keys to show and how to process them,
/// so we use a generic [ScreenController] here.
pub(crate) struct Screen<C: ScreenController> {
    info: Info,
    controller: C,
}

impl<C: ScreenController> Screen<C> {
    pub(super) fn new(controller: C) -> Self {
        Screen {
            info: Info::new_fixed(controller.init_text()),
            controller,
        }
    }

    pub(super) fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Max(10)].as_ref())
            .split(area);

        self.info.view(frame, layout[0]);
        command_pallet::view(frame, layout[1], &self.controller);
    }

    pub(super) fn on_event(&mut self, event: Event) -> ScreenFeedback {
        match event {
            Event::Key(key_event) => {
                let controller_ui_update = self.controller.on_event(key_event);
                let redraw_info = self.info.on_event(key_event);

                match controller_ui_update {
                    ScreenFeedback::None => {
                        if redraw_info {
                            ScreenFeedback::Redraw
                        } else {
                            ScreenFeedback::None
                        }
                    }
                    screen_feedback => screen_feedback,
                }
            }
            Event::Backend(BackendEvent::TaskCompleted(_, data)) => {
                self.info = match data {
                    Ok(x) => Info::new_scrollable(&x),
                    Err(e) => Info::new_error(&e),
                };
                ScreenFeedback::Redraw
            }
            _ => ScreenFeedback::None,
        }
    }
}

/// A screen controller is responsible for providing keystrokes information as
/// well as for dispatching keypress events. This is used as generic parameter
/// for [Screen] and it makes a difference between one screen or another.
pub(crate) trait ScreenController {
    fn name(&self) -> &'static str;

    fn init_text(&self) -> &'static str;

    fn command_keys(&self) -> &[ScreenCommandKey];

    fn toggle_keys(&self) -> &[ScreenToggleKey];

    /// Process key event, returning details on what's needed to be updated on
    /// UI.
    fn on_event(&mut self, key_event: KeyEvent) -> ScreenFeedback;
}

impl ScreenController for Box<dyn ScreenController> {
    fn name(&self) -> &'static str {
        self.deref().name()
    }

    fn init_text(&self) -> &'static str {
        self.deref().init_text()
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        self.deref().command_keys()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        self.deref().toggle_keys()
    }

    fn on_event(&mut self, key_event: KeyEvent) -> ScreenFeedback {
        self.deref_mut().on_event(key_event)
    }
}

type Keybinding = &'static str;

#[derive(Clone)]
pub(crate) struct ScreenCommandKey {
    pub keybinding: Keybinding,
    pub description: &'static str,
}

impl ScreenCommandKey {
    pub(crate) const fn new(keybinding: Keybinding, description: &'static str) -> Self {
        ScreenCommandKey {
            keybinding,
            description,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ScreenToggleKey {
    pub keybinding: Keybinding,
    pub description: &'static str,
    pub toggle: bool,
}

impl ScreenToggleKey {
    pub(crate) const fn new(keybinding: Keybinding, description: &'static str) -> Self {
        ScreenToggleKey {
            keybinding,
            description,
            toggle: false,
        }
    }
}

pub(crate) enum ScreenFeedback {
    NextScreen(Box<dyn ScreenController>),
    PreviousScreen(Box<dyn ScreenController>),
    Form(Box<dyn FormController>),
    Redraw,
    Quit,
    None,
}
