//! Application backend.
//! This includes all logic unrelated to UI.

mod identities;
mod insight;
mod state;
mod wallet;

use std::fmt::Display;

use rs_sdk::Sdk;
use serde::Serialize;
use tokio::sync::Mutex;

use self::identities::fetch_identity_by_b58_id;

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String),

    /// Variant for testing purposes
    RenderData(String),
}

pub(crate) enum BackendEvent {
    IdentityLoaded,
    IdentityUnloaded,
    TaskCompleted(Task, Result<String, String>),
}

pub(crate) struct Backend {
    sdk: Mutex<Sdk>,
}

impl Backend {
    pub(crate) fn new(sdk: Sdk) -> Self {
        Backend {
            sdk: Mutex::new(sdk),
        }
    }

    pub(crate) async fn run_task(&self, task: Task) -> Result<String, String> {
        match task {
            Task::FetchIdentityById(base58_id) => {
                let mut sdk = self.sdk.lock().await;
                fetch_identity_by_b58_id(&mut sdk, &base58_id).await
            }
            Task::RenderData(s) => Ok(s),
        }
    }
}

fn stringify_result<T: Serialize, E: Display>(result: Result<T, E>) -> Result<String, String> {
    match result {
        Ok(data) => {
            Ok(toml::to_string_pretty(&data).unwrap_or("Cannot serialize as TOML".to_owned()))
        }
        Err(e) => Err(e.to_string()),
    }
}
