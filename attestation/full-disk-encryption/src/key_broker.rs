use anyhow::{Ok, Result};
use core::fmt;
use reqwest::header::{HeaderMap, ACCEPT, CONTENT_TYPE};
use rustls::{
    cipher_suite::TLS13_AES_256_GCM_SHA384, version::TLS13, ClientConfig, OwnedTrustAnchor,
    RootCertStore,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrieveKeyRequest {
    pub quote: Vec<u8>,        // quote
    pub signed_nonce: Vec<u8>, // signed nonce from amber
    pub user_data: Vec<u8>,    // user application generated data
    pub event_log: Vec<u8>,    // TD event log
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrieveKeyResponse {
    pub wrapped_key: String,
    pub wrapped_swk: String,
}

pub async fn retreive_key_from_kbs(
    domain_name: &str,
    id: Uuid,
    req: &RetrieveKeyRequest,
) -> Result<RetrieveKeyResponse> {
    let url = format!("https://{}/kbs/v1/keys/{}/transfer", domain_name, id);

    let tls_config = default_cipher_suite_with_version()?;

    let builder = reqwest::ClientBuilder::new().use_preconfigured_tls(tls_config);

    let client = builder.build()?;

    let headers = default_request_headers()?;

    let resp: RetrieveKeyResponse = client
        .post(url)
        .headers(headers)
        .json(req)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

fn default_cipher_suite_with_version() -> Result<ClientConfig> {
    let suites = vec![TLS13_AES_256_GCM_SHA384];
    let versions = vec![&TLS13];
    let mut root_store = RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let tls_config = ClientConfig::builder()
        .with_cipher_suites(&suites)
        .with_safe_default_kx_groups()
        .with_protocol_versions(&versions)
        .expect("inconsistent cipher-suite/versions selected")
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(tls_config)
}

const HEADER_ATTESTATION_TYPE: &str = "Attestation-Type";

#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
enum AttestationType {
    SGX,
    TDX,
}

impl fmt::Display for AttestationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AttestationType::SGX => write!(f, "SGX"),
            AttestationType::TDX => write!(f, "TDX"),
        }
    }
}

fn default_request_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse()?);
    headers.insert(ACCEPT, "application/json".parse()?);
    headers.insert(
        HEADER_ATTESTATION_TYPE,
        AttestationType::TDX.to_string().parse()?,
    );
    Ok(headers)
}
