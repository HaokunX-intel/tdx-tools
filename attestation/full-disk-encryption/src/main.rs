use anyhow::{Result, Ok};
use base64::{Engine as _, engine::general_purpose};
use clap::Parser;

mod quote;
use quote::retrieve_quote;

mod ovmf_var;
use ovmf_var::retrieve_kbs_params;

mod key_broker;
use key_broker::retreive_key_from_kbs;

mod disk;
use disk::{crypt_setup, KEY_LENGTH};

mod td_report;

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

    // 1. get secret
    let secret = retrieve_kbs_params()?;
    let url = String::from_utf8(secret.url)?;
    println!("KBS Parmas Retrieved!");
    
    // 2. get quote
    let (_quote, id) = retrieve_quote()?;
    println!("TD Report & Quote Retrieved!");

    // 3. talk to kbs
    let retrieved_key = retreive_key_from_kbs(&url, id).await?;
    println!("Encryption Key Retrieved!");

    // 4. disk
    let key = &general_purpose::STANDARD
      .decode(retrieved_key.wrapped_key)
      .expect("Analyze Base64 Key Failed!");

    if key.len() != KEY_LENGTH {
        panic!("FDE Key not Support!");
    }
    
    crypt_setup(root.to_string(), name.to_string(), key);
    println!("Encryption Disk Mounted!");
  Ok(())
}

