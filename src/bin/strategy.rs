use std::{fs::File, panic, time::Duration};

use clap::{ArgAction, Parser};
use dpp::{identity::accessors::IdentityGettersV0, version::PlatformVersion};
use rs_platform_explorer::{
    backend::{self, identities::IdentityTask::{self}, insight::InsightAPIClient, wallet::WalletTask, Backend, Task},
    config::Config,
};
use rs_sdk::{RequestSettings, SdkBuilder};

#[derive(Parser, Debug)]
#[clap(about, long_about = None)]
struct Args {
    #[arg(short, long, help = "Specifies the stress test to run.")]
    test: Option<String>,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Enables state transition proof verification.")]
    prove: bool,

    #[arg(short, long, default_value_t = 20, help = "Specifies how many blocks to run the test. Default 20.")]
    blocks: u64,

    #[arg(short, long, help = "Specifies the minimum amount of Dash the loaded identity should have.")]
    dash: Option<u64>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logger
    let cli_action_taken = args.test.is_some();
    if cli_action_taken {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_writer(std::io::stdout)
            .with_ansi(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
    } else {
        let log_file = File::create("explorer.log").expect("create log file");

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_writer(log_file)
            .with_ansi(false)
            .finish();
    
        tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");    
    }

    // Test log statement
    tracing::info!("Logger initialized successfully");

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
    let config = Config::load();

    // Setup Platform SDK
    let address_list = config.dapi_address_list();
    let request_settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(10)),
        timeout: Some(Duration::from_secs(10)),
        retries: None,
        ban_failed_address: Some(false),
    };
    let sdk = SdkBuilder::new(address_list)
        .with_version(PlatformVersion::get(1).unwrap())
        .with_core(
            &config.core_host,
            config.core_rpc_port,
            &config.core_rpc_user,
            &config.core_rpc_password,
        )
        .with_settings(request_settings)
        .build()
        .expect("expected to build sdk");

    let insight = InsightAPIClient::new(config.insight_api_uri());

    let backend = Backend::new(sdk.as_ref(), insight.clone(), config.clone()).await;

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

    // Return if there's no loaded identity and the --dash flag is not set
    if backend.state().loaded_identity.lock().await.is_none() && args.dash.is_none() {
        tracing::error!("There's no loaded identity and the --dash argument is not passed");
        return
    }
    
    // Add loaded identity to known identities if it's not already there
    // And set selected_strategy to None
    {
        let state = backend.state();
        let loaded_identity = state.loaded_identity.lock().await;
        let mut selected_strategy = state.selected_strategy.lock().await;
        let mut known_identities = state.known_identities.lock().await;

        if let Some(loaded_identity) = loaded_identity.as_ref() {
            known_identities
                .entry(loaded_identity.id())
                .or_insert_with(|| loaded_identity.clone());
        }

        *selected_strategy = None;
    }
    
    // Handle CLI commands
    if let Some(start_dash) = args.dash {
        // Register identity with `dash` balance if there is none yet
        if backend.state().loaded_identity.lock().await.is_none() {
            let amount = start_dash * 100000000; // duffs to go into asset lock transaction

            tracing::info!(
                "Identity not registered, registering new identity with {} Dash",
                start_dash
            );

            backend
                .run_task(Task::Identity(IdentityTask::RegisterIdentity(amount)))
                .await;
        // Else, if there is a loaded identity, if the balance is less than start_dash, top it up
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
                "Platform wallet has {} Dash",
                balance as f64 / 100000000000.0
            );

            if balance < start_dash * 100000000000 {
                tracing::info!("Balance too low, adding {} more Dash", (start_dash as f64 * 100000000000.0 - balance as f64) / 100000000000.0);
                let amount = (start_dash * 100000000000 - balance) / 1000; // duffs to go into asset lock transaction
                backend.run_task(Task::Identity(IdentityTask::TopUpIdentity(amount))).await;
            }
        }
    }
    if let Some(test_name) = args.test {
        backend::strategies::run_strategy_task(
            &sdk,
            &backend.state(),
            backend::strategies::StrategyTask::RunStrategy(test_name.to_string(), args.blocks, args.prove),
            &insight,
        ).await;
    }
}