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
use dapi_grpc::platform::v0::get_identity_request;
use dapi_grpc::platform::v0::GetIdentityRequest;
use dapi_grpc::platform::v0::{
    get_data_contract_request, get_data_contracts_request,
    get_identity_balance_and_revision_request, get_identity_balance_request,
    get_identity_contract_nonce_request, get_identity_keys_request, get_identity_nonce_request,
    key_request_type, AllKeys, GetDataContractRequest, GetDataContractsRequest,
    GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceRequest,
    GetIdentityContractNonceRequest, GetIdentityKeysRequest, GetIdentityNonceRequest,
    KeyRequestType,
};
use dapi_grpc::tonic::transport::Uri;
use dapi_grpc::tonic::{Code, Status as TransportError};
use dashmap::DashMap;
use futures::future::{join_all, BoxFuture};
use futures::FutureExt;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use rand::seq::SliceRandom;
use rs_dapi_client::{
    Address, AddressList, DapiClient, DapiClientError, DapiRequest, DapiRequestExecutor,
    ExecutionResponse, ExecutionResult, IntoInner, RequestSettings,
};
use rs_platform_explorer::config::Config;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use tracing_subscriber::EnvFilter;

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
    // TODO: Configure report time
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

    let config = Config::load();

    let rate = Rate::new(args.rate, args.rate_unit);

    // TODO: It should be configurable and all request types should be added
    let requests = vec![
        // Existing DPNS identity
        GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(
                get_identity_request::GetIdentityRequestV0 {
                    id: dpp::system_data_contracts::dpns_contract::OWNER_ID_BYTES.to_vec(),
                    prove: true,
                },
            )),
        }.into(),
        // Existing DashPay identity
        GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(
                get_identity_request::GetIdentityRequestV0 {
                    id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES.to_vec(),
                    prove: true,
                },
            )),
        }.into(),
        // Existing Masternode reward shares identity
        GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(
                get_identity_request::GetIdentityRequestV0 {
                    id: dpp::system_data_contracts::masternode_reward_shares_contract::OWNER_ID_BYTES.to_vec(),
                    prove: true,
                },
            )),
        }.into(),
        // Non-existing identity
        GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(
                get_identity_request::GetIdentityRequestV0 {
                    id: vec![0; 32],
                    prove: false,
                },
            )),
        }.into(),
        // Get identity nonce
        GetIdentityNonceRequest {
            version: Some(get_identity_nonce_request::Version::V0(get_identity_nonce_request::GetIdentityNonceRequestV0 {
                identity_id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES.to_vec(),
                prove: true,
            })),
        }.into(),
        // Get identity contract nonce
        GetIdentityContractNonceRequest {
            version: Some(
                get_identity_contract_nonce_request::Version::V0(get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                    identity_id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES
                        .to_vec(),
                    contract_id: dpp::system_data_contracts::dashpay_contract::ID.to_vec(),
                    prove: true,
                },
            )),
        }.into(),
        // Get identity contract nonce
        GetIdentityContractNonceRequest {
            version: Some(
                get_identity_contract_nonce_request::Version::V0(get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                    identity_id: dpp::system_data_contracts::dpns_contract::OWNER_ID_BYTES
                        .to_vec(),
                    contract_id: dpp::system_data_contracts::dpns_contract::ID.to_vec(),
                    prove: true,
                },
            )),
        }.into(),
        // Non existing identity contract nonce
        GetIdentityContractNonceRequest {
            version: Some(
                get_identity_contract_nonce_request::Version::V0(get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                    identity_id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES
                        .to_vec(),
                    contract_id: vec![0; 32],
                    prove: false,
                },
            )),
        }.into(),
        // Get DashPay identity balance
        GetIdentityBalanceRequest {
            version: Some(get_identity_balance_request::Version::V0(get_identity_balance_request::GetIdentityBalanceRequestV0 {
                id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES
                    .to_vec(),
                prove: true,
            })),
        }.into(),
        // Get DashPay identity balance and revision
        GetIdentityBalanceAndRevisionRequest {
            version: Some(get_identity_balance_and_revision_request::Version::V0(get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0 {
                id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES
                    .to_vec(),
                prove: true,
            })),
        }.into(),
        GetIdentityKeysRequest {
            version: Some(get_identity_keys_request::Version::V0(get_identity_keys_request::GetIdentityKeysRequestV0 {
                identity_id: dpp::system_data_contracts::dashpay_contract::OWNER_ID_BYTES
                    .to_vec(),
                request_type: Some(KeyRequestType { request: Some(key_request_type::Request::AllKeys(AllKeys {})) }),
                limit: None,
                offset: None,
                prove: true,
            })),
        }.into(),
        // Dashpay contract
        GetDataContractRequest {
            version: Some(get_data_contract_request::Version::V0(get_data_contract_request::GetDataContractRequestV0 {
                id: dpp::system_data_contracts::dashpay_contract::ID
                    .to_vec(),
                prove: true,
            })),
        }.into(),
        // DPNS contract
        GetDataContractRequest {
            version: Some(get_data_contract_request::Version::V0(get_data_contract_request::GetDataContractRequestV0 {
                id: dpp::system_data_contracts::dpns_contract::ID
                    .to_vec(),
                prove: true,
            })),
        }.into(),
        // Masternode reward shares contract
        GetDataContractRequest {
            version: Some(get_data_contract_request::Version::V0(get_data_contract_request::GetDataContractRequestV0 {
                id: dpp::system_data_contracts::masternode_reward_shares_contract::ID
                    .to_vec(),
                prove: true,
            })),
        }.into(),
        // Non existing contract
        GetDataContractRequest {
            version: Some(get_data_contract_request::Version::V0(get_data_contract_request::GetDataContractRequestV0 {
                id: vec![0; 32],
                prove: false,
            })),
        }.into(),
        // Non existing contract
        GetDataContractsRequest {
            version: Some(get_data_contracts_request::Version::V0(get_data_contracts_request::GetDataContractsRequestV0 {
                ids: vec![dpp::system_data_contracts::dpns_contract::ID
                    .to_vec(), dpp::system_data_contracts::masternode_reward_shares_contract::ID
                    .to_vec(), vec![0; 32]],
                prove: true,
            })),
        }.into(),
    ];

    let request_plan = RequestPlan::new(requests);

    send_many_request_to_drive(
        &config,
        args.time.map(|t| Duration::from_secs(t.into())),
        args.connections,
        rate,
        request_plan,
    )
    .await;
}

async fn send_many_request_to_drive(
    config: &Config,
    duration: Option<Duration>,
    concurrent_connections: u16,
    rate: Rate,
    requests: RequestPlan,
) {
    let start_time = Instant::now();
    let cancel_test = CancellationToken::new();
    let rate = Arc::new(rate);
    let requests = Arc::new(requests);

    let duration_message = if let Some(duration) = duration {
        format!(" for {} seconds", duration.as_secs_f32())
    } else {
        String::new()
    };

    tracing::info!(
        "query {} identities with {} parallel connections{}",
        rate,
        concurrent_connections,
        duration_message
    );

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
        let report_summary = Arc::clone(&summary);

        tokio::spawn(async move {
            let mut report_interval = interval(Duration::from_secs(10));

            while !cancel_report.is_cancelled() {
                report_interval.tick().await;

                tracing::info!("{}", report_summary.report_message());
            }
        })
    };

    tasks.push(report_task);

    let mut addresses = AddressPool::from(config.dapi_addresses.as_str());

    for connection_id in 0..concurrent_connections {
        // TODO: Use SDK but it should be able to return TransportError
        // Pick an address obe by one for each connection
        // and create one client per connection
        let address_list = addresses.next_one_address_list();
        let client = DapiClient::new(address_list, request_settings);
        let connection_client = Arc::new(client);

        let connection_rate = Arc::clone(&rate);
        let cancel_connection = cancel_test.clone();
        let connection_summary = Arc::clone(&summary);
        let connections_requests = Arc::clone(&requests);

        // Send requests through the connection in a loop
        let connection_task = tokio::spawn(async move {
            while !cancel_connection.is_cancelled() {
                // Wait for the rate limiter to allow further processing
                tokio::select! {
                    _ = connection_rate.limiter.until_ready() => {},
                    _ = cancel_connection.cancelled() => return,
                }

                let request_client = Arc::clone(&connection_client);
                let request_summary = Arc::clone(&connection_summary);

                // Select a random request from the list
                let request = connections_requests.random();

                // Send a request without waiting for the response,
                // so we can send many requests in parallel through one connection
                tokio::spawn(async move {
                    let span = tracing::span!(
                        tracing::Level::TRACE,
                        "connection",
                        connection_id = connection_id
                    );

                    send_request(request_client, request_settings, request, &request_summary)
                        .instrument(span)
                        .await
                });
            }
        });

        tasks.push(connection_task);
    }

    join_all(tasks).await;

    tracing::info!("[DONE] {}", summary.report_message());
}

async fn send_request(
    client: Arc<DapiClient>,
    settings: RequestSettings,
    request: AnyDapiRequest,
    summary: &TestSummary,
) {
    // TODO: Record response duration and then report average and histogram?
    match request
        .execute(client.as_ref(), settings)
        .await
        .into_inner()
    {
        Ok(_) => summary.add_ok(),
        Err(DapiClientError::Transport(e, ..)) => summary.add_error(match e {
            rs_dapi_client::transport::TransportError::Grpc(status) => status,
        }),
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

// TODO: Move to crate and reuse in load test and strategy

struct RequestPlan {
    requests: Vec<AnyDapiRequest>,
}

impl RequestPlan {
    pub fn new(requests: Vec<AnyDapiRequest>) -> Self {
        if requests.is_empty() {
            panic!("request list must not be empty");
        }

        Self { requests }
    }

    pub fn random(&self) -> AnyDapiRequest {
        self.random_with_rng(&mut rand::thread_rng())
    }

    pub fn random_with_rng(&self, rng: &mut impl rand::Rng) -> AnyDapiRequest {
        self.requests
            .choose(rng)
            .expect("request list must not be empty")
            .clone()
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
enum AnyDapiRequest {
    GetIdentityRequest(GetIdentityRequest),
    GetIdentityNonceRequest(GetIdentityNonceRequest),
    GetIdentityContractNonceRequest(GetIdentityContractNonceRequest),
    GetIdentityBalanceRequest(GetIdentityBalanceRequest),
    GetIdentityBalanceAndRevisionRequest(GetIdentityBalanceAndRevisionRequest),
    GetIdentityKeysRequest(GetIdentityKeysRequest),
    GetDataContractRequest(GetDataContractRequest),
    GetDataContractsRequest(GetDataContractsRequest),
}

impl From<GetIdentityRequest> for AnyDapiRequest {
    fn from(value: GetIdentityRequest) -> Self {
        AnyDapiRequest::GetIdentityRequest(value)
    }
}

impl From<GetIdentityNonceRequest> for AnyDapiRequest {
    fn from(value: GetIdentityNonceRequest) -> Self {
        AnyDapiRequest::GetIdentityNonceRequest(value)
    }
}

impl From<GetIdentityContractNonceRequest> for AnyDapiRequest {
    fn from(value: GetIdentityContractNonceRequest) -> Self {
        AnyDapiRequest::GetIdentityContractNonceRequest(value)
    }
}

impl From<GetIdentityBalanceRequest> for AnyDapiRequest {
    fn from(value: GetIdentityBalanceRequest) -> Self {
        AnyDapiRequest::GetIdentityBalanceRequest(value)
    }
}

impl From<GetIdentityBalanceAndRevisionRequest> for AnyDapiRequest {
    fn from(value: GetIdentityBalanceAndRevisionRequest) -> Self {
        AnyDapiRequest::GetIdentityBalanceAndRevisionRequest(value)
    }
}

impl From<GetIdentityKeysRequest> for AnyDapiRequest {
    fn from(value: GetIdentityKeysRequest) -> Self {
        AnyDapiRequest::GetIdentityKeysRequest(value)
    }
}

impl From<GetDataContractRequest> for AnyDapiRequest {
    fn from(value: GetDataContractRequest) -> Self {
        AnyDapiRequest::GetDataContractRequest(value)
    }
}

impl From<GetDataContractsRequest> for AnyDapiRequest {
    fn from(value: GetDataContractsRequest) -> Self {
        AnyDapiRequest::GetDataContractsRequest(value)
    }
}

impl DapiRequest for AnyDapiRequest {
    type Response = ();

    fn execute<'c, D: DapiRequestExecutor>(
        self,
        dapi_client: &'c D,
        settings: RequestSettings,
    ) -> BoxFuture<'c, ExecutionResult<Self::Response, DapiClientError>>
    where
        Self: 'c,
    {
        match self {
            AnyDapiRequest::GetIdentityRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetIdentityNonceRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetIdentityContractNonceRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetIdentityBalanceRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetIdentityBalanceAndRevisionRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetIdentityKeysRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetDataContractRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
            AnyDapiRequest::GetDataContractsRequest(request) => request
                .execute(dapi_client, settings)
                .map(|result| {
                    result.map(|response| ExecutionResponse {
                        inner: (),
                        retries: response.retries,
                        address: response.address,
                    })
                })
                .boxed(),
        }
    }
}

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

struct Rate {
    rate: u32,
    unit: RateUnit,
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} per {}", self.rate, self.unit)
    }
}

impl Rate {
    pub fn new(rate: u32, unit: RateUnit) -> Self {
        let non_zero_rate = NonZeroU32::new(rate).expect("rate must be greater than zero");
        let quota = match unit {
            RateUnit::Second => Quota::per_second(non_zero_rate),
            RateUnit::Minute => Quota::per_minute(non_zero_rate),
        };

        let limiter = RateLimiter::direct(quota);

        Self {
            rate,
            unit,
            limiter,
        }
    }
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

        format!(
            "{elapsed_secs} secs passed. {total} processed ({rate} q/s): {oks} successful, {errors} failed{}",
            error_message
        )
    }
}

struct AddressPool {
    addresses: Vec<Address>,
    current_index: usize,
}

impl AddressPool {
    fn new(addresses: Vec<Address>) -> Self {
        Self {
            addresses,
            current_index: 0,
        }
    }

    fn next_address(&mut self) -> &Address {
        let address = self.addresses.get(self.current_index).unwrap();
        self.current_index = (self.current_index + 1) % self.addresses.len();
        address
    }

    fn next_one_address_list(&mut self) -> AddressList {
        let mut address_list = AddressList::new();
        address_list.add(self.next_address().clone());
        address_list
    }
}

impl From<&str> for AddressPool {
    fn from(addresses: &str) -> Self {
        let addresses = addresses
            .split(',')
            .map(|uri| {
                let uri = Uri::from_str(uri).expect("invalid uri");
                Address::from(uri)
            })
            .collect();

        Self::new(addresses)
    }
}
