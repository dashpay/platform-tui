//! UI layer.
//!
//! Platform Explorer's UI part on top level consists of two components: screen
//! and form. At a time only one of them will occupy the application's UI, both
//! explained in details in their modules.

mod form;
mod screen;
mod status_bar;
pub(crate) mod views;

use std::{mem, ops::Deref, time::Instant};

use dpp::identity::accessors::IdentityGettersV0;
use tuirealm::{
    terminal::TerminalBridge,
    tui::prelude::{Constraint, Direction, Layout},
};

use self::{
    form::{Form, FormController, FormStatus},
    screen::{Screen, ScreenController, ScreenFeedback},
    status_bar::StatusBarState,
    views::{main::MainScreenController, strategies::StrategiesScreenController},
};
use crate::{
    backend::{AppState, AppStateUpdate, BackendEvent, Task},
    Event,
};

pub struct IdentityBalance {
    pub credits: u64,
}

impl IdentityBalance {
    pub fn from_credits(credits: u64) -> Self {
        IdentityBalance { credits }
    }

    pub fn dash_str(&self) -> String {
        let dash_amount = self.credits as f64 * 10f64.powf(-11.0);
        format!("{:.4} DASH", dash_amount)
    }
}

/// TUI entry point that handles terminal events as well as terminal output,
/// linking UI parts together.
pub struct Ui {
    redraw_ts: Instant,
    terminal: TerminalBridge,
    status_bar_state: StatusBarState,
    screen: Screen<Box<dyn ScreenController>>,
    form: Option<Form<Box<dyn FormController>>>,
    blocked: bool,
    screen_stack: Vec<Screen<Box<dyn ScreenController>>>,
}

/// UI updates delivered to the main application loop.
pub enum UiFeedback {
    Redraw,
    Quit,
    ExecuteTask(Task),
    None,
}

impl Ui {
    pub fn redraw(&mut self) {
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
                self.status_bar_state.view(frame, layout[1]);
            })
            .expect("unable to draw to terminal");
    }

    pub fn new(initial_identity_balance: Option<IdentityBalance>) -> Self {
        let mut terminal = TerminalBridge::new().expect("cannot initialize terminal app");
        terminal
            .enter_alternate_screen()
            .expect("cannot put terminal into alt mode");
        terminal
            .enable_raw_mode()
            .expect("cannot enable terminal raw mode");

        let main_screen_controller = MainScreenController::new();

        let mut status_bar_state = initial_identity_balance
            .map(StatusBarState::with_balance)
            .unwrap_or_default();

        status_bar_state.add_child(main_screen_controller.name());

        let screen = Screen::new(Box::new(main_screen_controller) as Box<dyn ScreenController>);

        let mut ui = Ui {
            redraw_ts: Instant::now(),
            terminal,
            status_bar_state,
            screen,
            form: None,
            blocked: false,
            screen_stack: Vec::new(),
        };

        ui.redraw();
        ui
    }

    pub async fn on_event<'s>(
        &mut self,
        app_state: impl Deref<Target = AppState>,
        event: Event<'s>,
    ) -> UiFeedback {
        let mut redraw = false;

        // On task completion we shall unfreeze the screen and update status bar
        // "blocked" message
        if let Event::Backend(
            BackendEvent::TaskCompleted { .. }
            | BackendEvent::TaskCompletedStateChange { .. }
            | BackendEvent::StrategyCompleted { .. }
            | BackendEvent::StrategyError { .. },
        ) = &event
        {
            self.status_bar_state.unblock();
            self.blocked = false;
            redraw = true;
        }

        // A special treatment for loaded identity app state update: status bar should
        // be updated as well
        if let Event::Backend(
            BackendEvent::AppStateUpdated(AppStateUpdate::LoadedIdentity(identity))
            | BackendEvent::TaskCompletedStateChange {
                app_state_update: AppStateUpdate::LoadedIdentity(identity),
                ..
            },
        ) = &event
        {
            self.status_bar_state
                .update_balance(IdentityBalance::from_credits(identity.balance()));
            redraw = true;
        }

        // Identity cleared
        if let Event::Backend(
            BackendEvent::AppStateUpdated(AppStateUpdate::ClearedLoadedIdentity)
            | BackendEvent::TaskCompletedStateChange {
                app_state_update: AppStateUpdate::ClearedLoadedIdentity,
                ..
            },
        ) = &event
        {
            self.status_bar_state.clear_balance();
            redraw = true;
        }

        // Wallet cleared
        if let Event::Backend(
            BackendEvent::AppStateUpdated(AppStateUpdate::ClearedLoadedWallet)
            | BackendEvent::TaskCompletedStateChange {
                app_state_update: AppStateUpdate::ClearedLoadedWallet,
                ..
            },
        ) = &event
        {
            self.status_bar_state.clear_balance();
            redraw = true;
        }

        // On failed identity refresh we indicate that balance might be incorrect
        if let Event::Backend(
            BackendEvent::AppStateUpdated(AppStateUpdate::FailedToRefreshIdentity)
            | BackendEvent::TaskCompletedStateChange {
                app_state_update: AppStateUpdate::FailedToRefreshIdentity,
                ..
            },
        ) = &event
        {
            self.status_bar_state.set_balance_error();
        }

        // Update all the stacked screens with the relevant state
        if let Event::Backend(
            BackendEvent::AppStateUpdated(_) | BackendEvent::TaskCompletedStateChange { .. },
        ) = &event
        {
            for screen in self.screen_stack.iter_mut() {
                screen.on_event(&event);
            }
        }

        if self.blocked {
            return UiFeedback::None;
        }

        let ui_feedback = if let (Some(form), Event::Key(event)) = (&mut self.form, &event) {
            match form.on_event(*event) {
                FormStatus::Done { task, block } => {
                    self.form = None;
                    if block {
                        self.status_bar_state.block();
                        self.blocked = true;
                    }
                    UiFeedback::ExecuteTask(task)
                }
                FormStatus::NextScreen(controller_builder) => {
                    self.form = None;
                    let controller = controller_builder(app_state.deref()).await;
                    self.status_bar_state.add_child(controller.name());
                    let old_screen = mem::replace(&mut self.screen, Screen::new(controller));
                    self.screen_stack.push(old_screen);
                    UiFeedback::Redraw
                }
                FormStatus::PreviousScreen => {
                    self.form = None;
                    self.status_bar_state.to_parent();
                    if let Some(previous_screen) = self.screen_stack.pop() {
                        self.screen = previous_screen;
                    } else {
                        // Exit if no previous screen
                        return UiFeedback::Quit;
                    }
                    UiFeedback::Redraw
                }
                FormStatus::Redraw => UiFeedback::Redraw,
                FormStatus::None => UiFeedback::None,
                FormStatus::Exit => {
                    self.form = None;
                    UiFeedback::Redraw
                }
            }
        } else {
            match self.screen.on_event(&event) {
                ScreenFeedback::NextScreen(controller_builder) => {
                    let controller = controller_builder(app_state.deref()).await;
                    self.status_bar_state.add_child(controller.name());
                    let old_screen = mem::replace(&mut self.screen, Screen::new(controller));
                    self.screen_stack.push(old_screen);
                    UiFeedback::Redraw
                }
                ScreenFeedback::PreviousScreen => {
                    self.status_bar_state.to_parent();

                    let current_screen_name = self.screen.controller.name();
                    let previous_screen_name =
                        self.screen_stack.last().map(|s| s.controller.name());

                    if current_screen_name == "Strategy"
                        && previous_screen_name == Some("Strategies")
                    {
                        // Rebuild the StrategiesScreenController when navigating back from
                        // SelectedStrategyScreenController
                        let new_controller =
                            StrategiesScreenController::new(app_state.deref()).await;
                        self.screen_stack.pop(); // Remove the old StrategiesScreenController from the stack
                        self.screen = Screen::new(Box::new(new_controller));
                    } else {
                        // Regular back navigation
                        if let Some(previous_screen) = self.screen_stack.pop() {
                            self.screen = previous_screen;
                        } else {
                            // Exit if no previous screen
                            return UiFeedback::Quit;
                        }
                    }

                    UiFeedback::Redraw
                }
                ScreenFeedback::Form(controller) => {
                    self.form = Some(Form::new(controller));
                    UiFeedback::Redraw
                }
                ScreenFeedback::FormThenNextScreen { form, screen } => {
                    self.form = Some(Form::new(form));

                    let controller = screen(app_state.deref()).await;
                    self.status_bar_state.add_child(controller.name());
                    let old_screen = mem::replace(&mut self.screen, Screen::new(controller));
                    self.screen_stack.push(old_screen);

                    UiFeedback::Redraw
                }
                ScreenFeedback::Task { task, block } => {
                    if block {
                        self.status_bar_state.block();
                        self.blocked = true;
                    }
                    UiFeedback::ExecuteTask(task)
                }
                ScreenFeedback::Redraw => UiFeedback::Redraw,
                ScreenFeedback::Quit => UiFeedback::Quit,
                ScreenFeedback::None => UiFeedback::None,
            }
        };

        if matches!(ui_feedback, UiFeedback::None) && redraw {
            UiFeedback::Redraw
        } else {
            ui_feedback
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
