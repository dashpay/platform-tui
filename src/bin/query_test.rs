use std::fmt::Display;
use std::str::FromStr;
use std::{
    fmt,
    num::NonZeroU32,
    panic,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use clap::Parser;
use dapi_grpc::platform::v0::get_identity_request::{GetIdentityRequestV0, Version};
use dapi_grpc::platform::v0::GetIdentityRequest;
use dapi_grpc::tonic::transport::Uri;
use dapi_grpc::tonic::{Code, Status as TransportError};
use dashmap::DashMap;
use futures::future::join_all;
use governor::{Quota, RateLimiter};
use rs_dapi_client::{
    Address, AddressList, DapiClient, DapiClientError, DapiRequest, RequestSettings,
};
use rs_platform_explorer::config::Config;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use tracing_subscriber::EnvFilter;

#[derive(clap::ValueEnum, Clone, Debug)]
enum RateUnit {
    Second,
    Minute,
}

impl Display for RateUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateUnit::Second => write!(f, "second"),
            RateUnit::Minute => write!(f, "minute"),
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short,
        long,
        default_value_t = 1,
        help = "The number of connections to open simultaneously"
    )]
    connections: u16,
    #[arg(
        long,
        help = "Request rate unit",
        value_enum,
        default_value_t = RateUnit::Second
    )]
    rate_unit: RateUnit,
    #[arg(
        short,
        long,
        default_value_t = 50000,
        help = "Number of requests to send per unit"
    )]
    rate: u32,
    #[arg(
        long,
        short,
        help = "The duration (in seconds) for which to handle the load test"
    )]
    time: Option<u16>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logger
    {
        let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
            EnvFilter::try_from_env(rust_log).expect("invalid RUST_LOG filter")
        } else {
            EnvFilter::try_new("info,rs_dapi_client=off").expect("invalid default filter")
        };

        tracing_subscriber::fmt::fmt()
            .with_env_filter(env_filter)
            .init();
    }

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
            %location,
            "Panic occurred: {}",
            message
        );

        default_panic_hook(panic_info);
    }));

    // Load configuration
    let config = Config::load();

    query_identities(
        &config,
        args.time.map(|t| Duration::from_secs(t.into())),
        args.connections,
        args.rate,
        args.rate_unit,
    )
    .await;
}

async fn query_identities(
    config: &Config,
    duration: Option<Duration>,
    concurrent_connections: u16,
    rate: u32,
    rate_unit: RateUnit,
) {
    let start_time = Instant::now();
    let cancel_test = CancellationToken::new();

    let duration_message = if let Some(duration) = duration {
        format!(" for {} seconds", duration.as_secs_f32())
    } else {
        String::new()
    };

    tracing::info!(
        "query {} per {} non existing identities with {} parallel connections{}",
        rate,
        rate_unit,
        concurrent_connections,
        duration_message
    );

    // Rate limiter
    let rate_limiter = {
        let non_zero_rate = NonZeroU32::new(rate).expect("rate must be greater than zero");
        let quota = match rate_unit {
            RateUnit::Second => Quota::per_second(non_zero_rate),
            RateUnit::Minute => Quota::per_minute(non_zero_rate),
        };

        let rate_limiter = RateLimiter::direct(quota);

        Arc::new(rate_limiter)
    };

    // Cancel the token after the duration
    if let Some(duration) = duration {
        cancel_at(cancel_test.clone(), start_time + duration);
    }

    let request_settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(30)),
        timeout: Some(Duration::from_secs(30)),
        retries: Some(0),
        ban_failed_address: Some(false),
    };

    let summary = Arc::new(TestSummary::new(start_time));

    let mut tasks = Vec::new();

    // Report summary every 10 seconds
    let report_task = {
        let cancel_report = cancel_test.clone();
        let summary = Arc::clone(&summary);

        tokio::spawn(async move {
            let mut report_interval = interval(Duration::from_secs(10));

            while !cancel_report.is_cancelled() {
                report_interval.tick().await;

                tracing::info!("{}", summary.report_message());
            }
        })
    };

    tasks.push(report_task);

    let dapi_addresses: Vec<Address> = config
        .dapi_addresses
        .split(',')
        .map(|uri| {
            let uri = Uri::from_str(uri).expect("invalid uri");
            Address::from(uri)
        })
        .collect();

    for connection_id in 0..concurrent_connections {
        let rate_limiter = Arc::clone(&rate_limiter);
        let cancel_task = cancel_test.clone();
        let summary = Arc::clone(&summary);

        let connection_address = dapi_addresses
            .get(connection_id as usize % dapi_addresses.len())
            .expect("address must be present");

        let mut connection_address_list = AddressList::new();
        connection_address_list.add(connection_address.clone());

        let client = DapiClient::new(connection_address_list, request_settings);
        let client = Arc::new(client);

        let task = tokio::spawn(async move {
            while !cancel_task.is_cancelled() {
                // Wait for the rate limiter to allow further processing
                tokio::select! {
                    _ = rate_limiter.until_ready() => {},
                    _ = cancel_task.cancelled() => return,
                }

                let client = Arc::clone(&client);
                let summary = Arc::clone(&summary);

                tokio::spawn(async move {
                    let span = tracing::span!(
                        tracing::Level::TRACE,
                        "connection",
                        connection_id = connection_id
                    );

                    query_identity(client, request_settings, &summary)
                        .instrument(span)
                        .await
                });
            }
        });

        tasks.push(task);
    }

    join_all(tasks).await;

    tracing::info!("[DONE] {}", summary.report_message());
}

async fn query_identity(client: Arc<DapiClient>, settings: RequestSettings, summary: &TestSummary) {
    let request = GetIdentityRequest {
        version: Some(Version::V0(GetIdentityRequestV0 {
            id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES.to_vec(),
            prove: false,
        })),
    };

    match request.execute(client.as_ref(), settings).await {
        Ok(_) => summary.add_ok(),
        Err(DapiClientError::Transport(e, ..)) => summary.add_error(e),
        Err(e) => panic!("unexpected error: {}", e),
    }
}

fn cancel_at(cancellation_token: CancellationToken, deadline: Instant) {
    tokio::task::spawn(async move {
        tokio::select! {
            _ = cancellation_token.cancelled() => {},
            _ = tokio::time::sleep_until(deadline) => {},
        }

        cancellation_token.cancel()
    });
}

struct TestSummary {
    start_time: Instant,
    oks: AtomicU64,
    errors_per_code: DashMap<Code, AtomicU64>,
}

impl TestSummary {
    fn new(start_time: Instant) -> Self {
        Self {
            start_time,
            oks: Default::default(),
            errors_per_code: Default::default(),
        }
    }

    fn add_ok(&self) {
        self.oks.fetch_add(1, Ordering::Relaxed);
    }

    fn oks_count(&self) -> u64 {
        self.oks.load(Ordering::Relaxed)
    }

    fn add_error(&self, error: TransportError) {
        self.errors_per_code
            .entry(error.code())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    fn errors_count(&self) -> u64 {
        self.errors_per_code
            .iter()
            .map(|entry| entry.value().load(Ordering::Relaxed))
            .sum()
    }

    fn report_message(&self) -> String {
        let elapsed_secs = self.start_time.elapsed().as_secs();

        let oks = self.oks.load(Ordering::Relaxed);
        let mut errors = 0;
        let mut error_messages = Vec::new();

        for entry in self.errors_per_code.iter() {
            let code = entry.key();
            let count = entry.value().load(Ordering::Relaxed);

            errors += count;

            error_messages.push(format!("{:?} - {}", code, count));
        }

        let total = oks + errors;

        let rate = total.checked_div(elapsed_secs).unwrap_or(0);

        format!(
            "{elapsed_secs} secs passed. {total} processed ({rate} q/s): {oks} successful, {errors} failed: {}",
            error_messages.join(", ")
        )
    }
}
