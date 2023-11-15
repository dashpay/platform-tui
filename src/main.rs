// mod app;
mod backend;
// mod components;
// mod managers;
// mod mock_components;
mod ui;

use crossterm::event::{Event as TuiEvent, EventStream};
use dash_platform_sdk::SdkBuilder;
use dpp::version::PlatformVersion;
use futures::{future::OptionFuture, select, FutureExt, StreamExt};
use rs_dapi_client::AddressList;
use tuirealm::event::KeyEvent;

use self::{
    backend::{Backend, BackendEvent, Task},
    ui::{Ui, UiFeedback},
};

pub(crate) enum Event<'s> {
    Key(KeyEvent),
    Backend(BackendEvent<'s>),
}

#[tokio::main]
async fn main() {
    // Setup Platform SDK
    let mut address_list = AddressList::new();
    address_list.add_uri(rs_dapi_client::Uri::from_static(
        "https://44.239.39.153:1443",
    ));
    // address_list.add_uri(rs_dapi_client::Uri::from_static(
    //     "https://54.149.33.167:1443",
    // ));
    // address_list.add_uri(rs_dapi_client::Uri::from_static(
    //     "https://35.164.23.245:1443",
    // ));
    // address_list.add_uri(rs_dapi_client::Uri::from_static("https://52.33.28.47:1443"));
    let sdk = SdkBuilder::new(address_list)
        .with_version(PlatformVersion::get(1).unwrap())
        .with_core("127.0.0.1", 19998, "dashrpc", "password")
        .build()
        .expect("expected to build sdk");

    let mut ui = Ui::new();
    let backend = Backend::new(sdk).await;

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
                Some(backend_event) => Some(
                    Event::Backend(backend_event)
                ),
                None => None
            },
        };

        let ui_feedback = if let Some(e) = event {
            ui.on_event(backend.state(), e).await
        } else {
            UiFeedback::None
        };

        match ui_feedback {
            UiFeedback::Quit => active = false,
            UiFeedback::ExecuteTask(task) => {
                backend_task = Some(backend.run_task(task.clone()).boxed_local().fuse()).into();
                ui.redraw();
            }
            UiFeedback::Redraw => ui.redraw(), // TODO Debounce redraw?
            UiFeedback::None => (),
        }
    }
}
