// mod app;
mod backend;
// mod components;
// mod managers;
// mod mock_components;
mod ui;

use crossterm::event::{Event as TuiEvent, EventStream};
use futures::{future::OptionFuture, select, FutureExt, StreamExt};
use rs_sdk::SdkBuilder;

use self::{
    backend::{Backend, BackendEvent, Task},
    ui::{Event, Ui, UiFeedback},
};

#[tokio::main]
async fn main() {
    // Setup Platform SDK
    let sdk = SdkBuilder::new_mock()
        .build()
        .expect("cannot setup Platform SDK");

    let mut ui = Ui::new();
    let backend = Backend::new(sdk);

    let mut active = true;

    let mut terminal_event_stream = EventStream::new().fuse();
    let mut backend_task: OptionFuture<_> = None.into();

    while active {
        let event = select! {
            terminal_event = terminal_event_stream.next() => match terminal_event {
                None => panic!("terminal event stream closed unexpectedly"),
                Some(Err(_)) => panic!("terminal event stream closed unexpectedly"),
                Some(Ok(TuiEvent::Resize(_, _))) => {ui.redraw(); continue },
                Some(Ok(TuiEvent::Key(key_event))) => Some(Event::Key(key_event.into())),
                _ => None
            },
            backend_task_finished = backend_task => match backend_task_finished {
                Some((task, result)) => Some(
                    Event::Backend(BackendEvent::TaskCompleted(task, result))
                ),
                None => None
            },
        };

        let ui_feedback = event.map(|e| ui.on_event(e)).unwrap_or(UiFeedback::None);

        match ui_feedback {
            UiFeedback::Quit => active = false,
            UiFeedback::ExecuteTask(task) => {
                backend_task = Some(
                    backend
                        .run_task(task.clone())
                        .map(move |result| (task.clone(), result))
                        .boxed()
                        .fuse(),
                )
                .into()
            }
            UiFeedback::Redraw => ui.redraw(), // TODO Debounce redraw?
            UiFeedback::None => (),
        }
    }
}
