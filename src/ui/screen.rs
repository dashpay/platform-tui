//! Screen module.

pub(crate) mod utils;
pub(crate) mod widgets;

use std::ops::{Deref, DerefMut};

use futures::future::BoxFuture;
use tuirealm::{
    tui::prelude::{Constraint, Direction, Layout, Rect},
    Frame,
};

use self::widgets::command_pallet;
use super::{form::FormController, Event};
use crate::backend::{AppState, Task};

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
    controller: C,
}

impl<C: ScreenController> Screen<C> {
    pub(super) fn new(controller: C) -> Self {
        Screen { controller }
    }

    pub(super) fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Max(10)].as_ref())
            .split(area);

        self.controller.view(frame, layout[0]);
        command_pallet::view(frame, layout[1], &self.controller);
    }

    pub(super) fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        self.controller.on_event(event)
    }
}

/// A screen controller is responsible for providing keystrokes information as
/// well as for dispatching keypress events. This is used as generic parameter
/// for [Screen] and it makes a difference between one screen or another.
pub(crate) trait ScreenController {
    fn view(&mut self, frame: &mut Frame, area: Rect);

    fn name(&self) -> &'static str;

    fn command_keys(&self) -> &[ScreenCommandKey];

    fn toggle_keys(&self) -> &[ScreenToggleKey];

    /// Process key event, returning details on what's needed to be updated on
    /// UI.
    fn on_event(&mut self, event: &Event) -> ScreenFeedback;
}

impl ScreenController for Box<dyn ScreenController> {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.deref_mut().view(frame, area)
    }

    fn name(&self) -> &'static str {
        self.deref().name()
    }

    fn command_keys(&self) -> &[ScreenCommandKey] {
        self.deref().command_keys()
    }

    fn toggle_keys(&self) -> &[ScreenToggleKey] {
        self.deref().toggle_keys()
    }

    fn on_event(&mut self, event: &Event) -> ScreenFeedback {
        self.deref_mut().on_event(event)
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
    NextScreen(ScreenControllerBuilder),
    PreviousScreen,
    Form(Box<dyn FormController>),
    Task { task: Task, block: bool }, // TODO task should define whether it blocks or not
    Redraw,
    Quit,
    None,
}

pub(crate) type ScreenControllerBuilder =
    Box<dyn FnOnce(&AppState) -> BoxFuture<Box<dyn ScreenController>>>;
