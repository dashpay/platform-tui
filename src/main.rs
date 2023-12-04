mod backend;
mod config;
mod ui;

use std::{fs::File, panic, sync::Arc, time::Duration};

use crossterm::event::{Event as TuiEvent, EventStream};
use dash_platform_sdk::{mock::wallet::core_client::CoreClient, SdkBuilder};
use dpp::{identity::accessors::IdentityGettersV0, version::PlatformVersion};
use futures::{future::OptionFuture, select, FutureExt, StreamExt};
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook_tokio::Signals;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tuirealm::event::KeyEvent;
use ui::IdentityBalance;

use self::{
    backend::{Backend, BackendEvent, Task},
    ui::{Ui, UiFeedback},
};
use crate::{backend::insight::InsightAPIClient, config::Config};

const SHUTDOWN_TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));

pub(crate) enum Event<'s> {
    Key(KeyEvent),
    Backend(BackendEvent<'s>),
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // Initialize logger
    let log_file = File::create("explorer.log").expect("create log file");

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let subscriber = tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .with_writer(log_file)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("can't initialize logging");

    // Cancellation token that will be cancelled once SIGINT or SIGTERM is received.
    let cancel = CancellationToken::new();
    let _signal_handlers = install_signal_handlers(cancel.clone(), SHUTDOWN_TIMEOUT).await;

    // Log panics
    let default_panic_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .unwrap_or(&"unknown");

        let location = panic_info
            .location()
            .unwrap_or_else(|| panic::Location::caller());

        tracing::error!(
            location = tracing::field::display(location),
            "Panic occurred: {}",
            message
        );

        default_panic_hook(panic_info);
    }));

    // Load configuration
    let config = Config::load_testnet();

    // Setup Platform SDK
    let address_list = config.dapi_address_list();

    let sdk = SdkBuilder::new(address_list)
        .with_version(PlatformVersion::get(1).unwrap())
        .with_core(
            &config.core_host,
            config.core_port,
            &config.core_user,
            &config.core_password,
        )
        .build()
        .expect("expected to build sdk");

    let insight = InsightAPIClient::new(config.insight_api_uri());

    // We use core client to fetch quorum keys because we don't have SPV yet
    let core = CoreClient::new(
        &config.core_host,
        config.core_port,
        &config.core_user,
        &config.core_password,
    )
    .expect("expected to create core client");

    let backend = Arc::new(Backend::new(Arc::clone(&sdk), core, insight).await);
    sdk.set_context_provider(Arc::clone(&backend));
    let wallet = Arc::clone(&backend);
    sdk.set_wallet(wallet).await;

    let initial_identity_balance = backend
        .state()
        .loaded_identity
        .lock()
        .await
        .as_ref()
        .map(|identity| IdentityBalance::from_credits(identity.balance()));

    let mut ui = Ui::new(initial_identity_balance);

    let mut active = true;

    let mut terminal_event_stream = EventStream::new().fuse();
    let mut backend_task: OptionFuture<_> = None.into();

    let mut cancellation = Box::pin(cancel.cancelled()).fuse();
    while active {
        let event = select! {
            terminal_event = terminal_event_stream.next() => match terminal_event {
                None => panic!("terminal event stream closed unexpectedly"),
                Some(Err(_)) => panic!("terminal event stream closed unexpectedly"),
                Some(Ok(TuiEvent::Resize(_, _))) => {ui.redraw(); continue },
                Some(Ok(TuiEvent::Key(key_event))) => Some(Event::Key(key_event.into())),
                _ => None
            },
            backend_task_finished = backend_task => backend_task_finished.map(Event::Backend),
           _= cancellation => {
                active = false;
                tracing::debug!("received shutdown signal");
                None
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

/// Install signal handlers for SIGINT, SIGTERM and SIGQUIT.
///
/// Provided token will be cancelled once any of these signals is received.
///
/// Returns handler to manage signals and a task that will be spawned to handle
/// signals. Returned values must be in the scope of the application, otherwise
/// signals will not be handled.
async fn install_signal_handlers(
    token: CancellationToken,
    shutdown_timeout: Option<Duration>,
) -> (signal_hook_tokio::Handle, JoinHandle<()>) {
    let signals =
        Signals::new([SIGTERM, SIGINT, SIGQUIT]).expect("cannot initialize signal handler");

    let handle = signals.handle();
    let cancel = token.clone();

    let signals_task: JoinHandle<()> =
        tokio::spawn(handle_signals(signals, cancel, shutdown_timeout));

    (handle, signals_task)
}

async fn handle_signals(
    mut signals: Signals,
    cancel: CancellationToken,
    shutdown_timeout: Option<Duration>,
) {
    // we default to SIGINT exit code
    let mut exit_code = 128 + SIGINT;
    loop {
        tokio::select! {
            Some(signal) = signals.next() => {
                    tracing::info!(signal, "received signal");
                    exit_code = 128+signal;
                    cancel.cancel();
            },
            _= cancel.cancelled() => {
                tracing::info!(?shutdown_timeout, "shutdown requested");
                if let Some(timeout) = shutdown_timeout {
                    tokio::time::sleep(timeout).await;
                    tracing::info!(?timeout, "shutdown timed out, forcing shutdown");
                    signal_hook::low_level::exit(exit_code);
                }
            }
        }
    }
}
