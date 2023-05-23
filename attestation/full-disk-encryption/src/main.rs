use anyhow::{Ok, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use zeroize::Zeroize;

mod quote;
use quote::retrieve_quote;

mod ovmf_var;
use ovmf_var::retrieve_kbs_params;

mod key_broker;
use key_broker::{retreive_key_from_kbs, RetrieveKeyRequest};

mod disk;
use disk::{crypt_setup, KEY_LENGTH};

mod td_report;
mod verifier;

mod event_logs;
use event_logs::retrieve_event_logs;

mod amber;

use crate::key_broker::parse_retrieve_key_response;


#[derive(Parser)]
struct Args {
    // Boot partition with rootfs
    #[arg(short, long)]
    root: String,
    // rootfs name
    #[arg(short, long)]
    name: String,
}

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<()> {
    let args = Args::parse();
    let root = args.root;
    let name: String = args.name;

    // Generate a pair of rsa keys
    let rsa = openssl::rsa::Rsa::generate(3072)?;
    let key_pair = openssl::pkey::PKey::from_rsa(rsa)?;
    let key_pub_der = key_pair.public_key_to_der()?;
    let key_pub_sha512sum = openssl::sha::sha512(&key_pub_der);
    let key_pub_base64 = general_purpose::STANDARD.encode(key_pub_der.clone());
    println!("RSA Keys Generated!");

    // Get KBS params
    let secret = retrieve_kbs_params()?;
    let url = String::from_utf8(secret.url)?;
    println!("KBS Parmas Retrieved!");

    // Get quote
    let quote = retrieve_quote(key_pub_sha512sum)?;
    println!("TD Report & Quote Retrieved!");
    let quote_based64 = general_purpose::STANDARD.encode(quote);

    // Get event logs
    let _event_logs = retrieve_event_logs()?;
    println!("Event Logs Retrieved!");

    // Talk to kbs
    let retrieve_key_req = RetrieveKeyRequest {
        quote: quote_based64,
        user_data: key_pub_base64,
    };

    let resp = retreive_key_from_kbs(&url, secret.user_data.keyid, &retrieve_key_req).await?;
    println!("Encryption Key Retrieved!");

    // Disk mount
    let mut key = parse_retrieve_key_response(resp, key_pair).await?;

    if key.len() != KEY_LENGTH {
        panic!("FDE Key not Support!");
    }

    crypt_setup(root.to_string(), name.to_string(), &key);
    key.zeroize();
    println!("Encryption Disk Mounted!");
    Ok(())
}
