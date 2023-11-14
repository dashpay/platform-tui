//! Forms building blocks.
//! While a form could be defined fully manually this module provides utilities
//! to build forms of multiple inputs easily.

use tuirealm::{event::KeyEvent, tui::prelude::Rect, Frame};

use super::{Input, InputStatus};

/// A named [Input] to be used in [ComposedInput].
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

/// A type of input that combines several other inputs.
///
/// It displays one input at a time switching to the next one on
/// [InputStatus::Done] event from the current input. When finished a tuple of
/// all field results returned.
pub(crate) struct ComposedInput<F> {
    fields: F,
    index: u8,
}

impl<F> ComposedInput<F> {
    pub(crate) fn new(fields: F) -> Self {
        ComposedInput { fields, index: 0 }
    }

    pub(crate) fn step_index(&self) -> u8 {
        self.index
    }
}

/// Macro for internal use to implement [ComposedInput] for field combinations
/// up to 16 inputs. Because of the craving for type safety and a lack of tools
/// it's a common pattern in Rust to make use of tuples and declarative macros.
macro_rules! impl_sequential_input {
    // Macro entry point
    ($($input:ident),*) => {
        impl<$($input: Input),*> Input for ComposedInput<($(Field<$input>),*)> {
            #![allow(dead_code)]

            // ComposedInput's output is a tuple of all outputs, so
            // (Field<TextInput>, Field<TextInput>) will give us (String, String).
            type Output = ($($input::Output),*);

            fn on_event(&mut self, event: KeyEvent) -> InputStatus<Self::Output> {
                impl_sequential_input!{
                    @on_event_branch
                    self,
                    event,
                    (),
                    ($($input),*),
                    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }

            fn view(&mut self, frame: &mut Frame, area: Rect) {
                impl_sequential_input!{
                    @view_branch
                    self,
                    frame,
                    area,
                    ($($input),*),
                    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }
        }

        impl<$($input: Input),*> ComposedInput<($(Field<$input>),*)> {
            #![allow(dead_code)]

            pub(crate) fn step_name(&self) -> &'static str {
                impl_sequential_input!{
                    @step_name_branch
                    self,
                    ($($input),*),
                    (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
                }
            }

            pub(crate) fn steps_number(&self) -> u8 {
                impl_sequential_input!(@count 0, ($($input),*))
            }
        }
    };

    // @count_branch simply counts the arity of the tuple (number of fields)
    (@count
        $acc:expr,
        ($input:ident,
            $($rest:ident),*)
    ) => { impl_sequential_input!(@count $acc + 1, ($($rest),*)) };
    (@count $acc:expr, ($input:ident)) => { $acc + 1 };

    // delegates `view` method call to the current input depending on step index
    (@view_branch
        $self:ident,
        $frame:ident,
        $area:ident,
        ($input:ident,$($rest:ident),*),
        ($idx:tt, $($idx_rest:tt),*)
    ) => {
        if $self.index == $idx {
            return $self.fields.$idx.input.view($frame, $area);
        }

        impl_sequential_input!(@view_branch $self, $frame, $area, ($($rest),*), ($($idx_rest),*))
    };
    (@view_branch
        $self:ident,
        $frame:ident,
        $area:ident,
        ($input:ident),
        ($idx:tt, $($idx_rest:tt),*)
    ) => {
        $self.fields.$idx.input.view($frame, $area)
    };

    // delegates `step_name` method call to the current input depending on step index
    (@step_name_branch
        $self:ident,
        ($input:ident, $($rest:ident),*),
        ($idx:tt, $($idx_rest:tt),*)
    ) => {
        if $self.index == $idx {
            return $self.fields.$idx.name;
        }

        impl_sequential_input!(@step_name_branch $self, ($($rest),*), ($($idx_rest),*))
    };
    (@step_name_branch $self:ident, ($input:ident), ($idx:tt, $($idx_rest:tt),*)) => {
        $self.fields.$idx.name
    };

    // delegates `on_event` method call to the current input depending on step index
    (@on_event_branch
        $self:ident,
        $event:ident,
        ($($acc:expr),* $(,)?),
        ($input:ident, $($rest:ident),*),
        ($idx:tt, $($idx_rest:tt),*)
    ) => {
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

        impl_sequential_input! {
            @on_event_branch
            $self,
            $event,
            ($($acc,)* $self.fields.$idx.value.take().unwrap()),
            ($($rest),*),
            ($($idx_rest),*)
        }
    };
    (@on_event_branch
        $self:ident,
        $event:ident,
        ($($acc:expr),* $(,)?),
        ($input:ident),
        ($idx:tt, $($idx_rest:tt),*)
    ) => {
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

// Following macro calls implement [ComposedInput] for tuples of [Field]'s from
// 2 to 16. If a form made of only one field none if this machinery is really
// needed and form and its controller are pretty straightforward.
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
