use std::collections::HashMap;

use dapi_grpc::tonic::transport::Uri;
use dpp::dashcore::{Address, OutPoint, ScriptBuf, TxOut, Txid};

const ADDRESS_UTXO_PATH: &str = "addrs/utxo";

#[derive(Debug, thiserror::Error)]
#[error("insight error: {0}")]
pub(crate) struct InsightError(pub String);

#[derive(Debug, Clone)]
pub(crate) struct InsightAPIClient(Uri);

impl InsightAPIClient {
    pub fn new(uri: Uri) -> Self {
        Self(uri)
    }

    /// Fetches the unspent transaction outputs (UTXOs) with amounts for the
    /// specified addresses.
    ///
    /// # Arguments
    ///
    /// * `addresses` - A slice of references to `Address` objects for which
    ///   UTXOs are being requested.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - A `HashMap<OutPoint, TxOut>` where each `OutPoint` is a reference to a
    ///   UTXO and `TxOut` contains its details, if the operation is successful.
    /// - An `InsightError` if the request fails or the response cannot be
    ///   parsed.
    ///
    /// # Errors
    ///
    /// This method can return an `InsightError` in several cases, including:
    /// - Network errors during the request to the Insight API.
    /// - Non-successful HTTP status codes from the API response.
    /// - Failure to read the error body if the request is unsuccessful.
    /// - Missing fields in the JSON response (`txid`, `vout`, `satoshis`,
    ///   `scriptPubKey`).
    /// - Invalid formats for `txid` or `scriptPubKey`.
    pub async fn utxos_with_amount_for_addresses(
        &self,
        addresses: &[&Address],
    ) -> Result<HashMap<OutPoint, TxOut>, InsightError> {
        let url = format!("{}/{}", self.0, ADDRESS_UTXO_PATH);

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

            let vout = utxo
                .get("vout")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| InsightError("Missing or invalid vout".into()))?
                as u32;
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
}
