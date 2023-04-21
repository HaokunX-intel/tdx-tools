
use rustls::{cipher_suite::TLS13_AES_256_GCM_SHA384, version::TLS13, ClientConfig, RootCertStore, OwnedTrustAnchor};
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
    
    let tls_config = default_cipher_suite_with_version()?;
    
    let builder = reqwest::ClientBuilder::new()
        .use_preconfigured_tls(tls_config);
    
    // let client = reqwest::Client::new();
    let client = builder.build()?;

    let body = client.post(url)
        .send()
        .await?
        .text()
        .await?;
    let retrieved_key: RetrievedKey = serde_json::from_str(&body)?;
    Ok(retrieved_key)
}

fn default_cipher_suite_with_version() -> Result<ClientConfig> {
    let suites = vec![TLS13_AES_256_GCM_SHA384];
    let versions = vec![&TLS13];
    let mut root_store = RootCertStore::empty();
    root_store.add_server_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS
                .0
                .iter()
                .map(|ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }),
        );
    let tls_config = ClientConfig::builder()
        .with_cipher_suites(&suites)
        .with_safe_default_kx_groups()
        .with_protocol_versions(&versions)
        .expect("inconsistent cipher-suite/versions selected")
        .with_root_certificates(root_store)
        .with_no_client_auth();
    return Ok(tls_config);
}

