use dashcore_rpc::{Auth, Client, RpcApi};
use dashmap::DashMap;
use hdrhistogram::Histogram;
use rs_platform_explorer::config::Config;
use std::panic;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{interval, Instant};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_core_rpc_overload() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

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

    let addr = format!("http://{}:{}", config.core_host, config.core_rpc_port);
    let core_client = Client::new(
        &addr,
        Auth::UserPass(config.core_rpc_user, config.core_rpc_password),
    )
    .expect("failed to create Core RPC client");

    let core_client = Arc::new(core_client);

    let tip = core_client
        .get_block_count()
        .expect("failed to get block count");

    let summary = Arc::new(TestSummary::new());

    {
        let report_summary = Arc::clone(&summary);

        tokio::spawn(async move {
            let mut report_interval = interval(Duration::from_secs(10));

            loop {
                report_interval.tick().await;

                tracing::info!("{}", report_summary.report_message());
            }
        })
    };

    let permits = Arc::new(Semaphore::new(100));

    loop {
        let core_client = Arc::clone(&core_client);
        let permits = Arc::clone(&permits);
        let summary = Arc::clone(&summary);

        let permit = permits.acquire_owned().await.unwrap();

        tokio::task::spawn_blocking(move || {
            let id = "e8d89b4ddf987b5229e169d5dc2c19662d2e51363e4a73d6248147ac9dca32c0";
            let hash = "3d41a002f10c44e7a8540c14ce9cff9a950bdd0f6b01c64971bba805437da053";
            let signature = "8880dfe1935e97a4ced3b551aa5b33ddf6685287a110518d68ebb4b1f8f6c9a7195332fbe05c03f9fed14a6c2ab567120035ce460a6814ed18c988e36e430baa4343867e83870d463068d3b6b366947fd4006ad99c345befecc530f87e6999bc";

            let start_time = Instant::now();

            let result = core_client.get_verifyislock(id, hash, signature, Some(tip));

            match result {
                Ok(_) => summary.add_ok(start_time.elapsed()),
                Err(e) => summary.add_error(e),
            }

            drop(permit);
        });
    }
}

struct TestSummary {
    start_time: Instant,
    oks: AtomicU64,
    errors_per_code: DashMap<String, AtomicU64>,
    hist: parking_lot::Mutex<Histogram<u64>>,
}

impl TestSummary {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            oks: Default::default(),
            errors_per_code: Default::default(),
            hist: parking_lot::Mutex::new(
                Histogram::<u64>::new_with_bounds(1, 60 * 60 * 1000, 2).unwrap(),
            ),
        }
    }

    fn add_ok(&self, duration: Duration) {
        self.hist
            .lock()
            .record(duration.as_millis() as u64)
            .expect("value should be in range");
        self.oks.fetch_add(1, Ordering::Relaxed);
    }

    fn oks_count(&self) -> u64 {
        self.oks.load(Ordering::Relaxed)
    }

    fn add_error(&self, error: dashcore_rpc::Error) {
        self.errors_per_code
            .entry(error.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    fn errors_count(&self) -> u64 {
        self.errors_per_code
            .iter()
            .map(|entry| entry.value().load(Ordering::Relaxed))
            .sum()
    }

    // TODO: Drop intermediate values after report so we show only difference and then summary?
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

        let error_message = if !error_messages.is_empty() {
            format!(": {}", error_messages.join(", "))
        } else {
            String::new()
        };

        let mut hist_guard = self.hist.lock();

        let p50 = hist_guard.value_at_quantile(0.50);
        let p90 = hist_guard.value_at_quantile(0.90);
        let p99 = hist_guard.value_at_quantile(0.99);

        hist_guard.clear();

        drop(hist_guard);

        self.oks.store(0, Ordering::Relaxed);
        self.errors_per_code.clear();

        format!(
            "{elapsed_secs} secs passed. {total} processed ({rate} q/s): {oks} successful, {errors} failed{error_message}. Durations: p50={}s, p90={}s, p99={}s",
            p50 as f32 / 1000.0,
            p90 as f32 / 1000.0,
            p99 as f32 / 1000.0,
        )
    }
}
