use dpp::document::Document;
use dpp::identity::signer::Signer;
use rs_sdk::platform::DocumentQuery;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::Deref;
use std::{
    collections::HashSet,
    num::NonZeroU32,
    panic,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinSet;
use tokio::time::sleep;

use clap::Parser;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::StateTransition;
use dpp::{
    data_contract::{
        accessors::v0::{DataContractV0Getters, DataContractV0Setters},
        created_data_contract::CreatedDataContract,
        document_type::{
            accessors::DocumentTypeV0Getters,
            random_document::{CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType},
            DocumentType,
        },
        DataContract,
    },
    data_contracts::dpns_contract,
    document::DocumentV0Getters,
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, Identity, KeyType, Purpose,
        SecurityLevel,
    },
    platform_value::string_encoding::Encoding,
    state_transition::data_contract_create_transition::{
        methods::DataContractCreateTransitionMethodsV0, DataContractCreateTransition,
    },
    version::PlatformVersion,
};
use futures::future::join_all;
use governor::{Quota, RateLimiter};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rs_dapi_client::RequestSettings;
use rs_platform_explorer::{
    backend::{
        identities::IdentityTask, insight::InsightAPIClient, wallet::WalletTask, Backend, Task,
    },
    config::Config,
};
use rs_sdk::platform::transition::broadcast::BroadcastStateTransition;
use rs_sdk::{
    platform::{
        transition::{put_document::PutDocument, put_settings::PutSettings},
        Fetch, Identifier,
    },
    Sdk, SdkBuilder,
};
use simple_signer::signer::SimpleSigner;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short,
        long,
        help = "The number of connections to open to each endpoint simultaneously"
    )]
    connections: u16,
    #[arg(
        short,
        long,
        help = "The duration (in seconds) for which to handle the load test"
    )]
    time: u16,
    #[arg(
        short,
        long,
        help = "Number of transactions to send per second",
        default_value = "1"
    )]
    rate: u32,
    #[arg(
        long,
        help = "Number of contracts used to perform the test",
        default_value = "1"
    )]
    contracts: u32,

    #[arg(long, help = "The start nonce of identity contracts")]
    start_nonce: Option<u64>,

    #[arg(
        long,
        help = "How many contracts we want to push per block",
        default_value = "6"
    )]
    contract_push_speed: u32,

    #[arg(
        long,
        help = "How much we want to refill our wallet with in Dash if the balance is below this",
        default_value = "15"
    )]
    refill_amount: u64,

    #[arg(
        long,
        help = "How long to wait for the document to be mined, in seconds; 0 to disable waiting",
        default_value = "60"
    )]
    mine_timeout: u32,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logger
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

    // Setup Platform SDK
    let address_list = config.dapi_address_list();

    let sdk = SdkBuilder::new(address_list)
        .with_version(PlatformVersion::get(1).unwrap())
        .with_core(
            &config.core_host,
            config.core_rpc_port,
            &config.core_rpc_user,
            &config.core_rpc_password,
        )
        .build()
        .expect("expected to build sdk");

    let insight = InsightAPIClient::new(config.insight_api_uri());

    let backend = Backend::new(sdk.as_ref(), insight, config.clone()).await;

    // Create wallet if not initialized
    if backend.state().loaded_wallet.lock().await.is_none() {
        let Some(private_key) = config.wallet_private_key else {
            panic!("Wallet not initialized and no private key provided");
        };

        tracing::info!("Wallet not initialized, creating new wallet with configured private key");

        backend
            .run_task(Task::Wallet(WalletTask::AddByPrivateKey(
                private_key.clone(),
            )))
            .await;
    }

    // Refresh wallet core balance
    backend.run_task(Task::Wallet(WalletTask::Refresh)).await;

    // Register identity if there is no yet
    if backend.state().loaded_identity.lock().await.is_none() {
        let dash = 15;
        let amount = dash * 100000000; // Dash

        tracing::info!(
            "Identity not registered, registering new identity with {} Dash",
            dash
        );

        backend
            .run_task(Task::Identity(IdentityTask::RegisterIdentity(amount)))
            .await;
    } else {
        backend.run_task(Task::Wallet(WalletTask::Refresh)).await;
        backend
            .run_task(Task::Identity(IdentityTask::Refresh))
            .await;

        let balance = backend
            .state()
            .loaded_identity
            .lock()
            .await
            .as_ref()
            .unwrap()
            .balance();

        tracing::info!(
            "Credits in platform wallet have {} Dash",
            balance / 100000000000
        );

        if balance < args.refill_amount * 100000000000 {
            tracing::info!("Credits too low, adding {} more", args.refill_amount);
            let dash = args.refill_amount;
            let amount = dash * 100000000; // Dash
            let event = backend
                .run_task(Task::Identity(IdentityTask::TopUpIdentity(amount)))
                .await;
            tracing::info!("top up result: {:?}", event);
        }
    }

    let credits_balance = backend
        .state()
        .loaded_identity
        .lock()
        .await
        .as_ref()
        .unwrap()
        .balance();

    tracing::info!("Identity is initialized with {} credits", credits_balance);

    backend.state().save(&backend.config);

    let data_contract = DataContract::fetch(
        &backend.sdk,
        Into::<Identifier>::into(dpns_contract::ID_BYTES),
    )
    .await
    .unwrap()
    .unwrap();

    let document_type = data_contract
        .document_type_cloned_for_name("preorder")
        .unwrap()
        .into();

    let identity_lock = backend.state().loaded_identity.lock().await;
    let identity = identity_lock.as_ref().expect("no loaded identity");

    let identity_private_keys_lock = backend.state().identity_private_keys.lock().await;

    let mut signer = SimpleSigner::default();

    for (key_id, identity_public_key) in identity.public_keys() {
        let private_key = identity_private_keys_lock
            .get(&(identity.id(), *key_id))
            .expect("expected a private key")
            .clone();
        signer.add_key(identity_public_key.clone(), private_key);
    }

    let arc_signer = Arc::new(signer);

    // to be safe we need at least one contract per second of broadcasting,
    // so if we are aiming at 1000 tx/s we would need 1000 contracts

    let contract_count = args.contracts;

    let contracts = broadcast_contract_variants(
        Arc::clone(&sdk),
        &identity,
        arc_signer.clone(),
        &data_contract,
        contract_count,
        args.start_nonce,
        args.contract_push_speed,
    )
    .await;

    broadcast_random_documents_load_test(
        Arc::clone(&sdk),
        &identity,
        arc_signer,
        document_type,
        contracts,
        Duration::from_secs(args.time.into()),
        args.connections,
        args.rate,
        args.mine_timeout,
    )
    .await;
}

async fn broadcast_contract_variants(
    sdk: Arc<Sdk>,
    identity: &Identity,
    signer: Arc<SimpleSigner>,
    data_contract: &DataContract,
    count: u32,
    start_nonce: Option<IdentityNonce>,
    contract_push_speed: u32,
) -> Vec<DataContract> {
    let mut found_data_contracts = vec![];

    let mut count_left = count;

    let mut identity_nonce = sdk
        .get_identity_nonce(identity.id(), false, None)
        .await
        .expect("Couldn't get identity nonce");

    if let Some(start_nonce) = start_nonce {
        for nonce in start_nonce..=identity_nonce as u64 {
            let id = DataContract::generate_data_contract_id_v0(identity.id(), nonce);
            let maybe_contract = DataContract::fetch(&sdk, id).await;

            match maybe_contract {
                Ok(Some(contract)) => {
                    tracing::info!(
                        "data contract with id {} for nonce {} provably exists",
                        id,
                        nonce
                    );
                    found_data_contracts.push(contract);
                    count_left -= 1;
                }
                Ok(None) => {
                    tracing::info!(
                        "data contract with id {} for nonce {} provably does not exist, skipping",
                        id,
                        nonce
                    );
                }
                Err(e) => {
                    tracing::info!("ERROR!!!: getting data contract with id {} for nonce {} generated an error {:?}, skipping", id, nonce, e);
                }
            }

            if count_left == 0 {
                break;
            }
        }
    };

    if count_left > 0 {
        tracing::info!("we still need to register {} contracts", count_left);
    } else {
        return found_data_contracts;
    }

    tracing::info!(
        "registering data contracts, starting with nonce {}",
        identity_nonce + 1
    );
    let data_contract_variants = (0..count_left)
        .into_iter()
        .map(|_| {
            identity_nonce += 1;
            let new_id = DataContract::generate_data_contract_id_v0(identity.id(), identity_nonce);

            let mut data_contract_variant = data_contract.clone();
            data_contract_variant.set_id(new_id);
            CreatedDataContract::from_contract_and_identity_nonce(
                data_contract_variant,
                identity_nonce,
                PlatformVersion::latest(),
            )
            .expect("expected to get contract")
        })
        .collect::<Vec<_>>();

    let partial_identity = identity.clone().into_partial_identity_info();

    let key_to_use = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([SecurityLevel::CRITICAL]),
            HashSet::from([KeyType::ECDSA_SECP256K1]),
        )
        .expect("expected to get a key");

    let mut transitions_queue = VecDeque::new();

    let first_exists = DataContract::fetch(
        &sdk,
        data_contract_variants.first().unwrap().data_contract().id(),
    )
    .await
    .expect("expected to get data contract")
    .is_some();

    for data_contract_variant in data_contract_variants.iter().rev() {
        let exists = if !first_exists {
            false
        } else {
            DataContract::fetch(&sdk, data_contract_variant.data_contract().id())
                .await
                .expect("expected to get data contract")
                .is_some()
        };
        if !exists {
            let transition = DataContractCreateTransition::new_from_data_contract(
                data_contract_variant.data_contract().clone(),
                data_contract_variant.identity_nonce(),
                &partial_identity,
                key_to_use.id(),
                signer.as_ref(),
                PlatformVersion::latest(),
                None,
            )
            .expect("expected transition");

            transitions_queue.push_front(transition); // we push to the front on purpose
        } else {
            break; // if one exists we can assume all previous ones also exist
        }
    }

    for (i, transaction) in transitions_queue.iter().enumerate() {
        let StateTransition::DataContractCreate(DataContractCreateTransition::V0(v0)) =
            &transaction
        else {
            panic!("must be a data contract create transition")
        };
        let id = v0.data_contract.id();
        tracing::info!("registering contract {} with id {}", i, id);
        if i % contract_push_speed as usize == 0
            || i > (count - count % contract_push_speed) as usize
        {
            match transaction.broadcast_and_wait(&sdk, None).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::info!("Experienced a failure {:?} broadcasting a contract while waiting for the response", e);
                    sleep(Duration::from_secs(10)).await;
                    let contract_exists = DataContract::fetch(&sdk, id)
                        .await
                        .expect("expected to get data contract")
                        .is_some();
                    if contract_exists {
                        tracing::info!("contract proved to exist after 10 seconds");
                    } else {
                        tracing::info!("contract proved to not exist after 10 seconds");
                    }
                }
            }
        } else {
            match transaction.broadcast(&sdk).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::info!("Experienced a failure {:?} broadcasting a contract without waiting for the response", e);
                    sleep(Duration::from_secs(10)).await;
                    let contract_exists = DataContract::fetch(&sdk, id)
                        .await
                        .expect("expected to get data contract")
                        .is_some();
                    if contract_exists {
                        tracing::info!("contract proved to exist after 10 seconds");
                    } else {
                        tracing::info!("contract proved to not exist after 10 seconds");
                    }
                }
            }
        }
    }

    found_data_contracts.extend(
        data_contract_variants
            .into_iter()
            .map(|c| c.data_contract_owned()),
    );
    found_data_contracts
}

async fn broadcast_random_documents_load_test(
    sdk: Arc<Sdk>,
    identity: &Identity,
    signer: Arc<SimpleSigner>,
    document_type: DocumentType,
    contracts: Vec<DataContract>,
    duration: Duration,
    concurrent_requests: u16,
    rate_limit_per_sec: u32,
    mine_timeout_seconds: u32,
) {
    let rate_limit_per_sec = NonZeroU32::new(rate_limit_per_sec).unwrap_or(NonZeroU32::MAX);
    tracing::info!(
        document_type = document_type.name(),
        "broadcasting up to {} random documents per second in {} parallel threads for {} secs",
        rate_limit_per_sec,
        concurrent_requests,
        duration.as_secs_f32()
    );

    let identity_id = identity.id();
    let cancel = CancellationToken::new();

    let contracts = contracts
        .into_iter()
        .map(|c| Arc::new(c))
        .collect::<Vec<Arc<DataContract>>>();

    // Get identity public key
    let identity_public_key = identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            HashSet::from([document_type.security_level_requirement()]),
            HashSet::from([KeyType::ECDSA_SECP256K1, KeyType::BLS12_381]),
        )
        .expect("No public key matching security level requirements");

    // Created time for the documents

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis();

    // Generate and broadcast N documents

    let permits = Arc::new(Semaphore::new(concurrent_requests as usize));

    let rate_limit = Arc::new(RateLimiter::direct(Quota::per_second(rate_limit_per_sec)));

    // what the hell
    let oks = Arc::new(AtomicUsize::new(0)); // Atomic counter for tasks
    let errs = Arc::new(AtomicUsize::new(0)); // Atomic counter for tasks
    let pending = Arc::new(AtomicUsize::new(0));
    let last_report = Arc::new(AtomicU64::new(0));

    let start_time = Instant::now();

    let mut tasks = Vec::new();

    let settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(60)),
        timeout: Some(Duration::from_secs(30)),
        retries: Some(0),
        ban_failed_address: Some(false),
    };

    // start a timer to cancel the broadcast after the duration
    let timeout_cancel = cancel.clone();
    tokio::task::spawn(async move {
        tokio::select! {
            _ = timeout_cancel.cancelled() => {},
            _ = tokio::time::sleep_until(start_time + duration) => {},
        }

        tracing::info!("cancelling the broadcast of random documents");
        timeout_cancel.cancel()
    });

    let version = sdk.version();

    /*concurrent_requests: u16,
    rate_limit_per_sec: u32,
    mine_timeout_seconds: u32, */
    let docs_to_send: u64 = rate_limit_per_sec.get() as u64 * mine_timeout_seconds as u64;
    let docs_to_send = if docs_to_send > 10000 {
        10000
    } else {
        docs_to_send as usize
    };

    let checker = Arc::new(CheckWorker::new(
        sdk.clone(),
        docs_to_send,
        mine_timeout_seconds,
        &contracts,
        cancel.child_token(),
        signer.clone(),
    ));
    let std_rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    while !cancel.is_cancelled() {
        if cancel.is_cancelled() {
            break;
        }
        // Acquire a permit
        let permits = Arc::clone(&permits);
        let permit = permits.acquire_owned().await.unwrap();

        let oks = Arc::clone(&oks);
        let errs = Arc::clone(&errs);
        let pending = Arc::clone(&pending);
        let last_report = Arc::clone(&last_report);

        let identity_public_key = identity_public_key.clone();

        let signer = Arc::clone(&signer);

        let rate_limiter = rate_limit.clone();
        let cancel_task = cancel.clone();
        let sdk = Arc::clone(&sdk);

        let checker = checker.clone();
        let std_rng = std_rng.clone();

        let task = tokio::task::spawn(async move {
            // Wait for the rate limiter to allow further processing
            tokio::select! {
               _ = rate_limiter.until_ready() => {},
               _ = cancel_task.cancelled() => return,
            };

            // get some contract to use; it will be disposed (re-entered) when document is mined
            let start = Instant::now();
            let contract = checker.next().await.expect("contract reservation failed");
            if cancel_task.is_cancelled() {
                return;
            }
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 10 {
                tracing::trace!(
                    ?elapsed,
                    "contract reservation took too long, consider increasing --contracts argument"
                );
            };

            let document_type_to_use = Arc::new(
                contract
                    .document_type_cloned_for_name("preorder")
                    .expect("expected preorder document"),
            );

            let mut rng = std_rng.lock().await;
            let document_state_transition_entropy: [u8; 32] = rng.gen();
            let random_document = document_type_to_use
                .random_document_with_params(
                    identity_id,
                    document_state_transition_entropy.into(),
                    Some(created_at_ms as u64),
                    None,
                    None,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    &mut rng,
                    version,
                )
                .expect("expected a random document");
            drop(rng);

            // Broadcast the document
            tracing::trace!(
                "broadcasting document {}",
                random_document.id().to_string(Encoding::Base58),
            );

            pending.fetch_add(1, Ordering::SeqCst);

            let elapsed_secs = start_time.elapsed().as_secs();

            if start_time.elapsed().as_secs() % 10 == 0
                && elapsed_secs != last_report.load(Ordering::SeqCst)
            {
                let (mined, available_contracts) = (checker.mined(), checker.len());

                tracing::info!(
                    "{} secs passed: {} pending, {} broadcasted successfully, {} mined, {} failed; {} contracts available",
                    elapsed_secs,
                    pending.load(Ordering::SeqCst),
                    oks.load(Ordering::SeqCst),
                    mined,
                    errs.load(Ordering::SeqCst),
                    available_contracts
                  );
                last_report.swap(elapsed_secs, Ordering::SeqCst);
            }

            let start = Instant::now();
            let result = random_document
                .put_to_platform(
                    &sdk,
                    document_type_to_use.deref().clone(),
                    document_state_transition_entropy,
                    identity_public_key,
                    signer.as_ref(),
                    Some(PutSettings {
                        request_settings: settings,
                        identity_nonce_stale_time_s: None,
                        user_fee_increase: None,
                    }),
                )
                .await;
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 10 {
                tracing::info!(?elapsed, "put took too long");
            };
            // note we still hold the contract variant until we confirm the tx is included in a block
            drop(permit);

            pending.fetch_sub(1, Ordering::SeqCst);

            match result {
                Ok(st) => {
                    oks.fetch_add(1, Ordering::SeqCst);

                    tracing::trace!(
                        "document {} successfully broadcast",
                        random_document.id().to_string(Encoding::Base58),
                    );

                    let start = Instant::now();
                    checker.check(random_document, st, contract).await;
                    let elapsed = start.elapsed();
                    if elapsed.as_millis() > 1 {
                        tracing::info!(?elapsed, "check took too long");
                    };
                }

                Err(error) => {
                    tracing::error!(
                        ?error,
                        "failed to broadcast document {}: {}",
                        random_document.id().to_string(Encoding::Base58),
                        error
                    );

                    errs.fetch_add(1, Ordering::SeqCst);
                    checker.release(&[contract])
                }
            };
        });

        tasks.push(task)
    }

    join_all(tasks).await;

    let oks = oks.load(Ordering::SeqCst);
    let errs = errs.load(Ordering::SeqCst);
    let mined = checker.mined();

    tracing::info!(
        document_type = document_type.name(),
        "broadcasting {} random documents during {} secs. Broadcasted successfully: {}, failed: {}, mined: {}, rate: {} \
         docs/sec",
        oks + errs,
        duration.as_secs_f32(),
        oks,
        errs,
        mined,
        (oks + errs) as f32 / duration.as_secs_f32()
    );
}

#[derive(Clone, Debug)]
struct CheckRequest {
    doc: Document,
    dc: Arc<DataContract>,
    deadline: Instant,
}

/// Delayed verification of a document create state transition
struct CheckWorker<S: Signer> {
    done: mpsc::Sender<Arc<DataContract>>,
    available: Mutex<mpsc::Receiver<Arc<DataContract>>>,
    tasks: JoinSet<()>,
    sdk: Arc<Sdk>,
    cancel: CancellationToken,
    timeout: Duration,
    successful: Arc<AtomicU64>,
    _signer: Arc<S>,
    check_queue_tx: mpsc::Sender<CheckRequest>,
}

impl<S: Signer + 'static> CheckWorker<S> {
    /// new creates a new CheckWorker
    ///
    /// ## Arguments
    ///
    /// * `sdk` - Arc<Sdk> - The platform SDK
    /// * `mine_timeout_secs` - u32 - The time to wait for the document to be mined; 0 means no waiting
    /// * `contracts` - &[Arc<DataContract>] - The data contracts to be used
    /// * `cancel` - CancellationToken - The cancellation token
    /// * `signer` - Arc<S> - The signer used to sign the documents
    fn new(
        sdk: Arc<Sdk>,
        docs_to_send: usize,
        mine_timeout_secs: u32,
        contracts: &[Arc<DataContract>],
        cancel: CancellationToken,
        signer: Arc<S>,
    ) -> Self {
        const WORKERS: usize = 10;
        // When a contract is used by a task, it is removed from the channel.
        // When it's freed, it is put back into the channel.
        let (done, available) = mpsc::channel::<Arc<DataContract>>(contracts.len());
        let (check_queue_tx, check_queue_rx) = mpsc::channel(docs_to_send);
        let check_queue_rx = Arc::new(Mutex::new(check_queue_rx));

        let mut me = CheckWorker {
            done: done.clone(),
            available: Mutex::new(available),
            cancel,
            sdk,
            tasks: JoinSet::new(),
            successful: Arc::new(AtomicU64::new(0)),
            timeout: Duration::from_secs(mine_timeout_secs as u64),
            check_queue_tx,
            _signer: signer,
        };

        // put all contracts into the channel, so they are available
        me.release(contracts);

        // start the worker
        for thread_id in 0..WORKERS {
            me.tasks.spawn(Self::worker(
                thread_id,
                me.sdk.clone(),
                me.check_queue_tx.clone(),
                check_queue_rx.clone(),
                me.cancel.clone(),
                me.successful.clone(),
                me.done.clone(),
            ));
        }

        me
    }

    /// next returns the next data contract, or None on error
    async fn next(&self) -> Option<Arc<DataContract>> {
        let mut guard = self.available.lock().await;
        guard.recv().await
    }

    /// number of documents that were mined
    fn mined(&self) -> u64 {
        self.successful.load(Ordering::SeqCst)
    }

    /// number of available contracts to be acquired using [next()]
    fn len(&self) -> usize {
        self.done.max_capacity() - self.done.capacity()
    }

    /// release data contract(s) so that they can be retrieved using [next()]
    fn release(&self, data: &[Arc<DataContract>]) {
        for v in data {
            self.done
                .try_send(v.clone())
                .expect("doctype channel cannot be full");
        }
    }

    // Check if the state transition finished successfully.
    // Then release data contract so it can be used by another task.
    async fn check(&self, doc: Document, _st: StateTransition, dc: Arc<DataContract>) {
        let deadline = Instant::now() + self.timeout;
        let req = CheckRequest { dc, deadline, doc };

        if self.check_queue_tx.try_send(req.clone()).is_err() {
            tracing::warn!("check channel is full, blocking");
            self.check_queue_tx
                .send(req)
                .await
                .expect("check channel must always have enough space, data leak?");
        }
    }

    /// Worker function that waits for the document to be mined
    /// Returns where the document was mined or the timeout was reached.
    async fn worker(
        thread_id: usize,
        sdk: Arc<Sdk>,
        queue_tx: mpsc::Sender<CheckRequest>,
        queue_rx: Arc<Mutex<mpsc::Receiver<CheckRequest>>>,
        cancel: CancellationToken,
        included: Arc<AtomicU64>,
        done: mpsc::Sender<Arc<DataContract>>,
    ) {
        while !cancel.is_cancelled() {
            let check_request = {
                let mut guard = queue_rx.lock().await;
                let request = guard.recv().await.expect("checks queue must be open");
                drop(guard);
                request
            };

            let CheckRequest { doc, deadline, .. } = &check_request;
            let id = doc.id().to_string(Encoding::Base58);

            // Now query for individual document

            let query = DocumentQuery::new(check_request.dc.clone(), "preorder")
                .expect("create SdkDocumentQuery")
                .with_document_id(&doc.id());

            tokio::select! {
                _ = cancel.cancelled() => return,
                result = Document::fetch(&sdk, query) => {
                    match result {
                        Err(err) => {
                            if deadline.elapsed().is_zero() {
                                tracing::warn!(id, ?err, thread_id, "error when checking document, retrying");
                                queue_tx.send(check_request).await.expect("enqueue recheck");
                            } else {
                                tracing::error!(id, deadline=?deadline.elapsed(), thread_id, "error when checking document, not retrying");
                                done.try_send(check_request.dc).expect("done channel should always have enough capacity");
                            }
                        }
                        Ok(None) => {
                            if deadline.elapsed().is_zero() {
                                tracing::debug!(id, thread_id, "document not found, retrying");
                                queue_tx.send(check_request).await.expect("enqueue recheck of missing doc");
                            } else {
                                tracing::warn!(id, deadline=?deadline.elapsed(), thread_id, "timed out waiting for document to be mined");
                                done.try_send(check_request.dc).expect("done channel should always have enough capacity");
                            }
                        }
                        Ok(Some(_)) => {
                            tracing::trace!(id, thread_id, "document mined successfully");
                            included.fetch_add(1, Ordering::SeqCst);
                            done.try_send(check_request.dc).expect("done channel should always have enough capacity");
                        }
                    }
                }
            };
        }
    }
}
