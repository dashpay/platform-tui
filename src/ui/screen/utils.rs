//! Screen utilities.

/// Implements a `.builder()` function, that returns a closure expecting an
/// `&AppState` arg. This reduces a boilerplate required for every returned
/// screen controller, as it have to be a closure that returns a future that
/// resolves into dynamic screen controller.
macro_rules! impl_builder {
    ($screen:ty) => {
        impl $screen {
            pub(crate) fn builder() -> crate::ui::screen::ScreenControllerBuilder {
                use futures::FutureExt;

                Box::new(|app_state| {
                    async {
                        Box::new(<$screen>::new(app_state).await)
                            as Box<dyn crate::ui::screen::ScreenController>
                    }
                    .boxed()
                })
            }
        }
    };
}

macro_rules! impl_builder_no_args {
    ($screen:ty) => {
        impl $screen {
            #![allow(dead_code)]
            pub(crate) fn builder() -> crate::ui::screen::ScreenControllerBuilder {
                use futures::FutureExt;

                Box::new(|_| {
                    async {
                        Box::new(<$screen>::new()) as Box<dyn crate::ui::screen::ScreenController>
                    }
                    .boxed()
                })
            }
        }
    };
}

pub(crate) use impl_builder;
pub(crate) use impl_builder_no_args;
