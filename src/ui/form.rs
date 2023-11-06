//! Form component defintion.

mod completing_input;
mod text_input;
mod utils;

use std::ops::{Deref, DerefMut};

use tui_realm_stdlib::Label;
use tuirealm::{
    event::KeyEvent,
    tui::prelude::{Constraint, Direction, Layout, Rect},
    Frame, MockComponent,
};
pub(crate) use utils::{ComposedInput, Field};

pub(crate) use self::text_input::TextInput;
use crate::backend::Task;

/// Trait of every component suitable for processing user input.
/// Serves as a building block of a form as one may require several of them
/// and usually is context-aware unlike an input.
pub(crate) trait Input {
    type Output;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output>;

    fn view(&mut self, frame: &mut Frame, area: Rect);
}

/// [Input] result of processing a key event.
pub(crate) enum InputStatus<T> {
    /// Input is complete and a value is returned
    Done(T),
    /// Input is incomplete, but requires a view update
    Redraw,
    /// Input is incomplete and the key event was discarded
    None,
}

/// Form is a component that is responsible for handling key events and drawing
/// inputs accordingly. The generic parameter separates one form from another.
pub(crate) struct Form<C: FormController> {
    controller: C,
}

impl<C: FormController> Form<C> {
    pub(crate) fn new(controller: C) -> Self {
        Form { controller }
    }

    pub(crate) fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        self.controller.on_event(event)
    }

    pub(crate) fn view(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(2), Constraint::Min(10)].as_ref())
            .split(area);

        Label::default()
            .text(format!(
                "{}: {} [{} / {}]",
                self.controller.form_name(),
                self.controller.step_name(),
                self.controller.step_index() + 1,
                self.controller.steps_number()
            ))
            .view(frame, layout[0]);
        self.controller.step_view(frame, layout[1]);
    }
}

/// Type similar to [InputStatus], but represents the status of the whole form.
/// Unlike a single input, a form made of many inputs and uses a
/// [FormController] to process all of the results to produce a [Task] to
/// return, since a user's input precedes some action.
pub(crate) enum FormStatus {
    Done(Task),
    Redraw,
    None,
}

/// Similar to [crate::ui::ScreenController], a generic form knows how to draw
/// itself, but all specifics including how to process the input data are yet to
/// be defined, thus a controller used to finalize a form type.
pub(crate) trait FormController {
    /// Process a key event
    fn on_event(&mut self, event: KeyEvent) -> FormStatus;

    /// The form title
    fn form_name(&self) -> &'static str;

    /// Draw current input
    fn step_view(&mut self, frame: &mut Frame, area: Rect);

    /// Current step title
    fn step_name(&self) -> &'static str;

    /// Current step index
    fn step_index(&self) -> u8;

    /// Number of all form steps
    fn steps_number(&self) -> u8;
}

impl FormController for Box<dyn FormController> {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        self.deref_mut().on_event(event)
    }

    fn step_view(&mut self, frame: &mut Frame, area: Rect) {
        self.deref_mut().step_view(frame, area)
    }

    fn form_name(&self) -> &'static str {
        self.deref().form_name()
    }

    fn step_name(&self) -> &'static str {
        self.deref().step_name()
    }

    fn step_index(&self) -> u8 {
        self.deref().step_index()
    }

    fn steps_number(&self) -> u8 {
        self.deref().steps_number()
    }
}
