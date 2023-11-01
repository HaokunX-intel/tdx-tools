use std::vec;

use anyhow::{Ok, bail};

use crate::tpm2_tools::{run_tpm2_create_ak, run_tpm2_quote, run_tpm2_checkquote, run_tpm2_read_public};

const EK_CERT_CTX_INDEX: &str = "0x81010016";
const NONCE: &str = "abcef12345";

pub fn verify_tpm_quote() -> anyhow::Result<()> {
    // ek primary handle
    let ek_primary_handle: &str = "primary.handle";
    let handle_args: Vec<String> = vec![
        "-c".to_string(), EK_CERT_CTX_INDEX.to_string(),
        "-t".to_string(), ek_primary_handle.to_string(), 
    ];
    let output = run_tpm2_read_public(Some(handle_args))?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Create EK Primary Handle Failed!")
    }
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: EK Primary Handle Created!");

    // create ak
    let ak_ctx: &str = "ak.ctx";
    let ak_pub_pem: &str = "akpub.pem";
    let ak_name: &str = "ak.name";
    let ak_args: Vec<String> = vec![
        "-C".to_string(), ek_primary_handle.to_string(),
        "-c".to_string(), ak_ctx.to_string(),
        "-G".to_string(), "rsa".to_string(),
        "-s".to_string(), "rsassa".to_string(),
        "-g".to_string(), "sha256".to_string(),
        "-u".to_string(), ak_pub_pem.to_string(),
        "-f".to_string(), "pem".to_string(),
        "-n".to_string(), ak_name.to_string(),
    ];
    let output = run_tpm2_create_ak(Some(ak_args))?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Create AK Failed!")
    }
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: AK Created!");

    // create tpm quote
    let quote_msg: &str = "quote.msg";
    let quote_sig: &str = "quote.sig";
    let quote_pcrs: &str = "quote.pcrs";
    let quote_gen_args: Vec<String> = vec![
        "-c".to_string(), ak_ctx.to_string(), 
        "-l".to_string(), "sha256:15,16,22".to_string(), 
        "-q".to_string(), NONCE.to_string(), 
        "-m".to_string(), quote_msg.to_string(),
        "-s".to_string(), quote_sig.to_string(),
        "-o".to_string(), quote_pcrs.to_string(),
        "-g".to_string(), "sha256".to_string(),
    ];
    let output = run_tpm2_quote(Some(quote_gen_args))?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Create TPM Quote Failed!")
    }
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: TPM Quote Created!");

    // verify tpm quote
    let quote_verify_args: Vec<String> = vec![
        "-u".to_string(), ak_pub_pem.to_string(),
        "-m".to_string(), quote_msg.to_string(), 
        "-s".to_string(), quote_sig.to_string(), 
        "-f".to_string(), quote_pcrs.to_string(), 
        "-g".to_string(), "sha256".to_string(), 
        "-q".to_string(), NONCE.to_string(),
    ];
    let output = run_tpm2_checkquote(Some(quote_verify_args))?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Verify TPM Quote Failed!")
    }
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: TPM Quote Verified!");

    #[cfg(not(feature="DEBUG"))]
    {
        std::fs::remove_file(ek_primary_handle)?;
        std::fs::remove_file(ak_ctx)?;
        std::fs::remove_file(ak_pub_pem)?;
        std::fs::remove_file(ak_name)?;
        std::fs::remove_file(quote_msg)?;
        std::fs::remove_file(quote_pcrs)?;
        std::fs::remove_file(quote_sig)?;
    }
    Ok(())
}