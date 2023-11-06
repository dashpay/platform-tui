//! UI layer.
//!
//! Platform Explorer's UI part on top level consists of two components: screen
//! and form. At a time only one of them will occupy the application's UI, both
//! explained in details in their modules.

mod form;
mod screen;
mod status_bar;
mod views;

use tuirealm::{
    event::KeyEvent,
    terminal::TerminalBridge,
    tui::prelude::{Constraint, Direction, Layout},
    Frame,
};

use self::{
    form::{Form, FormController, FormStatus},
    screen::{Screen, ScreenController, ScreenFeedback},
    status_bar::StatusBarState,
    views::main_view::MainScreenController,
};
use crate::{BackendEvent, Task};

/// TUI entry point that handles terminal events as well as terminal output,
/// linking UI parts together.
pub(crate) struct Ui {
    terminal: TerminalBridge,
    status_bar_state: StatusBarState,
    screen: Screen<Box<dyn ScreenController>>,
    form: Option<Form<Box<dyn FormController>>>,
}

/// UI updates delivered to the main application loop.
pub(crate) enum UiFeedback {
    Redraw,
    Quit,
    ExecuteTask(Task),
    None,
}

impl Ui {
    pub(crate) fn redraw(&mut self) {
        self.terminal
            .raw_mut()
            .draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(10), Constraint::Max(3)].as_ref())
                    .split(frame.size());

                if let Some(form) = &mut self.form {
                    form.view(frame, layout[0]);
                } else {
                    self.screen.view(frame, layout[0])
                };
                status_bar::view(frame, layout[1], &self.status_bar_state);
            })
            .expect("unable to draw to terminal");
    }

    pub(crate) fn new() -> Self {
        let mut terminal = TerminalBridge::new().expect("cannot initialize terminal app");
        terminal
            .enter_alternate_screen()
            .expect("cannot put terminal into alt mode");
        terminal
            .enable_raw_mode()
            .expect("cannot enable terminal raw mode");

        let mut status_bar_state = StatusBarState::default();
        let main_screen_controller = MainScreenController;

        status_bar_state.add_child(main_screen_controller.name());

        let screen = Screen::new(Box::new(main_screen_controller) as Box<dyn ScreenController>);

        let mut ui = Ui {
            terminal,
            status_bar_state,
            screen,
            form: None,
        };

        ui.redraw();
        ui
    }

    pub(crate) fn on_event(&mut self, event: Event) -> UiFeedback {
        if let (Some(form), Event::Key(event)) = (&mut self.form, &event) {
            match form.on_event(*event) {
                FormStatus::Done(task) => {
                    self.form = None;
                    UiFeedback::ExecuteTask(task)
                }
                FormStatus::Redraw => UiFeedback::Redraw,
                FormStatus::None => UiFeedback::None,
            }
        } else {
            match self.screen.on_event(event) {
                ScreenFeedback::NextScreen(controller) => {
                    self.status_bar_state.add_child(controller.name());
                    self.screen = Screen::new(controller);
                    UiFeedback::Redraw
                }
                ScreenFeedback::PreviousScreen(controller) => {
                    self.status_bar_state.to_parent();
                    self.screen = Screen::new(controller);
                    UiFeedback::Redraw
                }
                ScreenFeedback::Form(controller) => {
                    self.form = Some(Form::new(controller));
                    UiFeedback::Redraw
                }
                ScreenFeedback::Redraw => UiFeedback::Redraw,
                ScreenFeedback::Quit => UiFeedback::Quit,
                ScreenFeedback::None => UiFeedback::None,
            }
        }
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        let _ = self.terminal.leave_alternate_screen();
        let _ = self.terminal.disable_raw_mode();
        let _ = self.terminal.clear_screen();
    }
}

pub(crate) enum Event {
    Key(KeyEvent),
    Backend(BackendEvent),
}
