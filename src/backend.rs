//! Application backend.
//! This includes all logic unrelated to UI.

mod identities;

use std::fmt::Display;

use rs_dapi_client::DapiClient;
use serde::Serialize;
use tokio::sync::Mutex;

use self::identities::fetch_identity_by_b58_id;

#[derive(Clone, PartialEq)]
pub(crate) enum Task {
    FetchIdentityById(String),
}

pub(crate) enum BackendEvent {
    IdentityLoaded,
    IdentityUnloaded,
    TaskCompleted(Task, Result<String, String>),
}

pub(crate) struct Backend {
    dapi_client: Mutex<DapiClient>,
}

impl Backend {
    pub(crate) fn new(dapi_client: DapiClient) -> Self {
        Backend {
            dapi_client: Mutex::new(dapi_client),
        }
    }

    pub(crate) async fn run_task(&self, task: Task) -> Result<String, String> {
        match task {
            Task::FetchIdentityById(base58_id) => {
                let mut dapi_client = self.dapi_client.lock().await;
                stringify_result(fetch_identity_by_b58_id(&mut dapi_client, base58_id).await)
            }
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
