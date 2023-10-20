use anyhow::{bail, Ok};
use x509_parser::{oid_registry::{asn1_rs::oid, Oid}, prelude::X509Certificate};
use ring::digest::{self, digest};
use base64::{Engine as _, engine::general_purpose};
use clap::{command, Arg};

mod utils;
mod tpm2_tools;
mod cert;
mod quote;

use cert::{
    retrieve_ca_cert_der, 
    retrieve_ek_cert_der, 
    from_der_file, 
    retrieve_cert_meta, 
    retrieve_raw_public_key};
use quote::ecdsa_quote_verification;

const CA_CERT_BASE64: &str = "ca-cert-base64";

const QUOTE_OID: Oid<'_> = oid!(2.16.840.1.113741.1.5.5.2.2);
const SHA384_BYTE_LEN: usize = 48;

fn verify_ca<'a>(ca_cert_der: &'a [u8]) -> anyhow::Result<X509Certificate<'a>> {
    let ca_cert = from_der_file(ca_cert_der)?;
    // 1. ca pub & quote in ca cert
    let ca_pub = ca_cert.public_key();
    let quote = ca_cert.get_extension_unique(&QUOTE_OID)?
        .map_or_else(
            || bail!("Retrieve quote failed!"),
            |v| Ok(v.value)
        )?;
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: Quote & CA Pubkey Retrieved");

    // 2. verify quote
    let digest_expected: &[u8] = ecdsa_quote_verification(&quote)
        .map_or_else(
            |e| bail!("Verify quote failed! Err: {:#?}", e),
            |_| {
                let td_report: &[u8] = &quote[48..(48+584)];
                let td_report_data: &[u8] = &td_report[520..(520+64)];
                Ok(&td_report_data[0..SHA384_BYTE_LEN])
            }
        )?;
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: Quote Verified");

    // 3. verify ca pub
    let ca_pub_raw = retrieve_raw_public_key(ca_pub)?;
    let digest_ret = digest(&digest::SHA384, &ca_pub_raw);

    if digest_expected.len() != digest_ret.as_ref().len() {
        bail!("Verify CA public key failed: length mismatch td_report_data {:} & digest {:}!", 
            digest_expected.len(), 
            digest_ret.as_ref().len()
        )
    }
    let cmp_zip = digest_expected.iter()
        .zip(digest_ret.as_ref().iter());
    for (t, d) in cmp_zip {
        if t != d {
            bail!("Verify CA public key failed: byte mismatch!")
        }
    }
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: CA Public Key Verified");

    // 4. verify ca cert    
    ca_cert.verify_signature(None)?;
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: CA Cert Verified");
    Ok(ca_cert)
}

fn main() -> anyhow::Result<()> {

    let matches = command!()
        .arg(
            Arg::new(CA_CERT_BASE64)
                .short('c')
                .long(CA_CERT_BASE64)
                .help("Verify the provided ca certification")   
        )
        .get_matches();
    if let Some(encoded) = matches.get_one::<String>(CA_CERT_BASE64) {
        let ca_cert_der = general_purpose::STANDARD.decode(encoded)?;
        if let Err(e) = verify_ca(&ca_cert_der) {
            bail!("Verify provided CA failed {:}!", e)
        }
        println!("verify_provided_ca: {:}", true);
        return Ok(())
    }

    // 0. ca cert & ek cert
    let cert_meta = retrieve_cert_meta()?;
    if cert_meta.seg_sizes.len() < 2 {
        bail!("Invalide indices length {:} < 2!", cert_meta.seg_sizes.len())
    }
    // 1. verify ca
    let ca_cert_der = retrieve_ca_cert_der(&cert_meta)?;
    let ca_cert = verify_ca(&ca_cert_der)?;

    // 2. verify ek
    let ca_pub = ca_cert.public_key();
    let ek_cert_der = retrieve_ek_cert_der(&cert_meta)?;
    let ek_cert = from_der_file(&ek_cert_der)?;
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: Cert Retrieved");
    ek_cert.verify_signature(Some(ca_pub))?;
    #[cfg(feature="DEBUG")]
    println!("SUCCESS: EK Cert Verified");

    // 3. output ek
    let ek_pub = ek_cert.public_key();
    let ek_pub_raw = retrieve_raw_public_key(ek_pub)?;
    let ek_pub_base64: String = general_purpose::STANDARD.encode(ek_pub_raw);
    println!("ek_pub_base64: {:}", ek_pub_base64);
    
    Ok(())
}
