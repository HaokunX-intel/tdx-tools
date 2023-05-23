use anyhow::{Ok, Result};
use openssl::{pkey::{Private,PKey}, rsa::Padding, symm::{Mode, Crypter}, hash::MessageDigest};
use core::fmt;
use std::sync::Arc;
use reqwest::{header::{HeaderMap, ACCEPT, CONTENT_TYPE}, Client, ClientBuilder};
use rustls::{
    cipher_suite::TLS13_AES_256_GCM_SHA384, version::TLS13, ClientConfig, OwnedTrustAnchor,
    RootCertStore,
};
use serde::{Deserialize, Serialize};

use crate::verifier::NoVerifier;
use base64::{engine::general_purpose, Engine as _};

use openssl::symm::Cipher;

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrieveKeyRequest {
    pub quote: String,        // quote
    // pub signed_nonce: Option<AmberNonceResponse>, // signed nonce from amber
    pub user_data: String,    // user application generated data
    // pub event_log: Option<Vec<u8>>,    // TD event log
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetrieveKeyResponse {
    pub wrapped_key: String,
    pub wrapped_swk: String,
}

pub async fn retreive_key_from_kbs(
    domain_name: &str,
    id: String,
    req: &RetrieveKeyRequest,
) -> Result<RetrieveKeyResponse> {
    let url = format!("https://{}/kbs/v1/keys/{}/transfer", domain_name, id);
    // Example:
    // let url = "https://pandora.intel.com/kbs/v1/keys/154e1c32-7d5b-43bb-b2b7-504b8daf8fff/transfer";

    let headers = default_request_headers()?;

    // Note: 
    // In production env, we should use always enable certification validation and 
    // use TLS13 with algorithm TLS13_AES_256_GCM_SHA384 to establish a tls channel.
    // However, due to the lack of the valid certification from the local KBS and 
    // support of the TLS13 protocol from the Amber attestation service. We disable
    // the certification validation for demo.
    //
    // Example for production env:
    // let tls_config = default_cipher_suite_with_version()?;
    // let builder = reqwest::ClientBuilder::new()
    //     .use_preconfigured_tls(tls_config);
    let builder = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true);
    let client = builder.build()?;
    
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

const U32_BYTE_SIZE: usize = 4;

pub async fn parse_retrieve_key_response(resp: RetrieveKeyResponse, key: PKey<Private>)-> Result<Vec<u8>> {
    let wrapped_key = general_purpose::STANDARD.decode(resp.wrapped_key)?;
    let wrapped_swk = general_purpose::STANDARD.decode(resp.wrapped_swk)?;
    
    // swk
    let mut rsa_decrypter = openssl::encrypt::Decrypter::new(&key)?;
    rsa_decrypter.set_rsa_padding(Padding::PKCS1_OAEP)?;
    rsa_decrypter.set_rsa_oaep_md(MessageDigest::sha256())?;

    let swk_buff_len = rsa_decrypter.decrypt_len(&wrapped_swk)?;
    let mut swk = vec![0u8;swk_buff_len];
    let swk_len = rsa_decrypter.decrypt(&wrapped_swk, &mut swk)?;
    swk.truncate(swk_len);

    
    // header
    let mut wrapped_key_offset = 0;
    let iv_length_len = U32_BYTE_SIZE;
    let iv_length = u32::from_ne_bytes(wrapped_key[wrapped_key_offset..wrapped_key_offset+iv_length_len].try_into()?) as usize;
    println!("iv_length: {:}", iv_length);

    wrapped_key_offset += iv_length_len;
    let tag_length_len = U32_BYTE_SIZE;
    let tag_length = u32::from_ne_bytes(wrapped_key[wrapped_key_offset..wrapped_key_offset+tag_length_len].try_into()?) as usize;
    println!("tag_length: {:}", tag_length);

    wrapped_key_offset += tag_length_len;
    let data_length_len = U32_BYTE_SIZE;
    let data_length = u32::from_ne_bytes(wrapped_key[wrapped_key_offset..wrapped_key_offset+data_length_len].try_into()?) as usize;
    println!("data_length: {:}", data_length);

    // iv
    wrapped_key_offset += data_length_len;
    let iv = wrapped_key[wrapped_key_offset..wrapped_key_offset+iv_length].to_vec();

    // data
    // encrypted text
    wrapped_key_offset += iv_length;
    let encrypted_key_length= data_length - tag_length;
    let encrypted_key = &wrapped_key[wrapped_key_offset..wrapped_key_offset+encrypted_key_length];

    wrapped_key_offset += encrypted_key_length;
    let tag = &wrapped_key[wrapped_key_offset..wrapped_key_offset+tag_length];

    let mut disk_key = vec![0u8;32];
    let mut cryptor = Crypter::new(Cipher::aes_256_gcm(), 
        Mode::Decrypt, 
        &swk, Some(&iv))?;
    cryptor.set_tag(tag)?;
    let mut cnt = cryptor.update(encrypted_key, &mut disk_key)?;
    cnt += cryptor.finalize(&mut disk_key[cnt..])?;
    disk_key.truncate(cnt);
    
    Ok(disk_key.to_vec())
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

    let mut tls_config = ClientConfig::builder()
        .with_cipher_suites(&suites)
        .with_safe_default_kx_groups()
        .with_protocol_versions(&versions)
        .expect("inconsistent cipher-suite/versions selected")
        .with_root_certificates(root_store)
        .with_no_client_auth();
    tls_config.dangerous().set_certificate_verifier(Arc::new(NoVerifier));
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



