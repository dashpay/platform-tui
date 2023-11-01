//! Form component defintion.

mod completing_input;
mod text_input;

use std::ops::{Deref, DerefMut};

use tui_realm_stdlib::Label;
use tuirealm::{
    event::KeyEvent,
    tui::prelude::{Constraint, Direction, Layout, Rect},
    Frame, MockComponent,
};

pub(crate) use self::text_input::TextInput;
use crate::backend::Task;

pub(crate) enum InputStatus<T> {
    Done(T),
    Redraw,
    None,
}

pub(crate) trait Input {
    type Output;

    fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output>;

    fn view(&mut self, frame: &mut Frame, area: Rect);
}

#[derive(Clone)]
pub(crate) struct Field<I: Input> {
    name: &'static str,
    input: I,
    value: Option<I::Output>,
}

impl<I: Input> Field<I> {
    pub(crate) fn new(name: &'static str, input: I) -> Self {
        Field {
            name,
            input,
            value: None,
        }
    }
}

pub(crate) struct SequentialInput<F> {
    fields: F,
    index: u8,
}

macro_rules! impl_sequential_input {
    ($($input:ident),*) => {
        impl<$($input: Input),*> Input for SequentialInput<($(Field<$input>),*)> {
            type Output = ($($input::Output),*);

            fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
                impl_sequential_input!{
                    @on_event_branch self, event, (), ($($input),*), (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }

            fn view(&mut self, frame: &mut Frame, area: Rect) {
                impl_sequential_input!{
                    @view_branch self, frame, area, ($($input),*), (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }
        }

        impl<$($input: Input),*> SequentialInput<($(Field<$input>),*)> {
            pub(crate) fn step_name(&self) -> &'static str {
                impl_sequential_input!{
                    @step_name_branch self, ($($input),*), (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }

            pub(crate) fn step_index(&self) -> u8 {
                self.index
            }

            pub(crate) fn steps_number(&self) -> u8 {
                impl_sequential_input!(@count 0, ($($input),*))
            }
        }
    };

    (@count $acc:expr, ($input:ident, $($rest:ident),*)) => { impl_sequential_input!(@count $acc + 1, ($($rest),*)) };
    (@count $acc:expr, ($input:ident)) => { $acc + 1 };

    (@view_branch $self:ident, $frame:ident, $area:ident, ($input:ident, $($rest:ident),*), ($idx:tt, $($idx_rest:tt),*)) => {
        if $self.index == $idx {
            return $self.fields.$idx.input.view($frame, $area);
        }

        impl_sequential_input!(@view_branch $self, $frame, $area, ($($rest),*), ($($idx_rest),*))
    };

    (@view_branch $self:ident, $frame:ident, $area:ident, ($input:ident), ($idx:tt, $($idx_rest:tt),*)) => {
        $self.fields.$idx.input.view($frame, $area)
    };

    (@step_name_branch $self:ident, ($input:ident, $($rest:ident),*), ($idx:tt, $($idx_rest:tt),*)) => {
        if $self.index == $idx {
            return $self.fields.$idx.name;
        }

        impl_sequential_input!(@step_name_branch $self, ($($rest),*), ($($idx_rest),*))
    };

    (@step_name_branch $self:ident, ($input:ident), ($idx:tt, $($idx_rest:tt),*)) => {
        $self.fields.$idx.name
    };

    (@on_event_branch $self:ident, $event:ident, ($($acc:expr),* $(,)?), ($input:ident, $($rest:ident),*), ($idx:tt, $($idx_rest:tt),*)) => {
        if $self.index == $idx {
            return match $self.fields.$idx.input.on_event($event) {
                InputStatus::Done(value) => {
                    $self.fields.$idx.value = Some(value);
                    $self.index += 1;
                    InputStatus::Redraw
                }
                InputStatus::Redraw => InputStatus::Redraw,
                InputStatus::None => InputStatus::None,
            }
        }

        impl_sequential_input! { @on_event_branch $self, $event, ($($acc,)* $self.fields.$idx.value.take().unwrap()), ($($rest),*), ($($idx_rest),*) }
    };

    (@on_event_branch $self:ident, $event:ident, ($($acc:expr),* $(,)?), ($input:ident), ($idx:tt, $($idx_rest:tt),*)) => {
        return match $self.fields.$idx.input.on_event($event) {
            InputStatus::Done(value) => {
                $self.fields.$idx.value = Some(value);
                $self.index += 1;
                InputStatus::Done(($($acc,)* $self.fields.$idx.value.take().unwrap()))
            }
            InputStatus::Redraw => InputStatus::Redraw,
            InputStatus::None => InputStatus::None,
        }
    };
}

impl_sequential_input!(I1, I2);
impl_sequential_input!(I1, I2, I3);
impl_sequential_input!(I1, I2, I3, I4);
impl_sequential_input!(I1, I2, I3, I4, I5);
impl_sequential_input!(I1, I2, I3, I4, I5, I6);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15);
impl_sequential_input!(I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15, I16);

impl<F> SequentialInput<F> {
    pub(crate) fn new(fields: F) -> Self {
        SequentialInput { fields, index: 0 }
    }
}

pub(crate) enum FormStatus {
    Done(Task),
    Redraw,
    None,
}

pub(crate) trait FormController {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus;

    fn view(&mut self, frame: &mut Frame, area: Rect);

    fn form_name(&self) -> &'static str;

    fn step_name(&self) -> &'static str;

    fn step_index(&self) -> u8;

    fn steps_number(&self) -> u8;
}

impl FormController for Box<dyn FormController> {
    fn on_event(&mut self, event: KeyEvent) -> FormStatus {
        self.deref_mut().on_event(event)
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.deref_mut().view(frame, area)
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
        self.controller.view(frame, layout[1]);
    }
}
