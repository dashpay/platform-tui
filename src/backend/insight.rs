use std::collections::HashMap;

use dpp::dashcore::{Address, OutPoint, ScriptBuf, TxOut, Txid};

const ADDRESS_UTXO_PATH: &str = "addrs/utxo";
const INSIGHT_URL: &str = "https://insight.dash.org/insight-api-dash";
const INSIGHT_FAILOVER_URL: &str = "https://insight.dash.show/api";
const TESTNET_INSIGHT_URL: &str = "https://insight.testnet.networks.dash.org:3002/insight-api";

#[derive(Debug, thiserror::Error)]
#[error("insight error: {0}")]
pub(crate) struct InsightError(pub String);

pub(crate) async fn utxos_with_amount_for_addresses(
    addresses: &[&Address],
    is_mainnet: bool,
) -> Result<HashMap<OutPoint, TxOut>, InsightError> {
    let insight_url = if is_mainnet {
        INSIGHT_URL
    } else {
        TESTNET_INSIGHT_URL
    };
    match utxos(insight_url, addresses).await {
        Ok(result) => Ok(result),
        Err(_) => {
            let insight_backup_url = if is_mainnet {
                INSIGHT_FAILOVER_URL
            } else {
                TESTNET_INSIGHT_URL
            };
            utxos(insight_backup_url, addresses).await
        }
    }
}

async fn utxos(
    insight_url: &str,
    addresses: &[&Address],
) -> Result<HashMap<OutPoint, TxOut>, InsightError> {
    let url = format!("{}/{}", insight_url, ADDRESS_UTXO_PATH);

    let addr_str = addresses
        .iter()
        .map(|address| address.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let resp = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("addrs={}", addr_str))
        .send()
        .await
        .map_err(|e| InsightError(e.to_string()))?;

    let status = resp.status();

    if !status.is_success() {
        let error_body = resp
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error body".to_string());
        return Err(InsightError(format!(
            "Request failed with status {}: {}",
            status, error_body
        )));
    }

    let json: Vec<serde_json::Value> =
        resp.json().await.map_err(|e| InsightError(e.to_string()))?;
    let mut utxos = HashMap::new();
    for utxo in json.iter() {
        let txid_str = utxo
            .get("txid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InsightError("Missing txid".into()))?;
        let txid =
            Txid::from_hex(txid_str).map_err(|_| InsightError("Invalid txid format".into()))?;

        let vout =
            utxo.get("vout")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| InsightError("Missing or invalid vout".into()))? as u32;
        let value = utxo
            .get("satoshis")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| InsightError("Missing or invalid amount".into()))?;

        let script_buf_str = utxo
            .get("scriptPubKey")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InsightError("Missing scriptPubKey".into()))?;
        let script = ScriptBuf::from_hex(script_buf_str)
            .map_err(|_| InsightError("Invalid scriptPubKey format".into()))?;

        utxos.insert(
            OutPoint { txid, vout },
            TxOut {
                value,
                script_pubkey: script,
            },
        );
    }

    Ok(utxos)
}
