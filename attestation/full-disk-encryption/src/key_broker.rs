use serde::{Deserialize, Serialize};
use anyhow::{Result, Ok};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrievedKeyRequest {
    pub quote: String,          // quote
    pub signed_nonce: String,   // signed nonce from amber
    pub user_data: String,      // user application generated data
    pub event_log: String,      // TD event log
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrievedKey {
    pub wrapped_key: String,
    pub wrapped_swk: String,
}

pub async fn retreive_key_from_kbs(domain_name: &str, id: Uuid) ->Result<RetrievedKey> {
    let url = format!("https://{}/kbs/v1/keys/{}/transfer", domain_name, id);
    let client = reqwest::Client::new();
    let body = client.post(url)
        .send()
        .await?
        .text()
        .await?;
    let retrieved_key: RetrievedKey = serde_json::from_str(&body)?;
    Ok(retrieved_key)
}

