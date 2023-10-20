use std::fs;

use anyhow::{bail, Ok, Result};
use x509_parser::{prelude::{X509Certificate, FromDer}, nom::AsBytes, public_key::PublicKey, x509::SubjectPublicKeyInfo};

use crate::tpm2_tools::{self, run_tpm2_nvread, run_tpmnvread_public};

const INDEX_PREFIX: &str = "0x1c0010";
const EK_INDEX: &str = "0x01c00016";

pub struct CertMeta {
    pub seg_sizes: Vec<i32>
}

pub fn from_der_file(der: &[u8]) -> Result<X509Certificate> {
    let (_, cert) = X509Certificate::from_der(der.as_bytes())?;
    Ok(cert)
}

pub fn retrieve_cert_meta() -> anyhow::Result<CertMeta> {
    let output = run_tpmnvread_public(None)?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Retrieve Certificate Meta Failed!")
    }
    Ok(CertMeta{
        seg_sizes: tpm2_tools::parse_meta(output.stdout)?
    })
}

pub fn retrieve_ca_cert_der(cert_meta: &CertMeta) -> anyhow::Result<Vec<u8>> {
    let seg_sizes = cert_meta.seg_sizes[1..].to_vec();
    let ca_cert = merge_ca_cert_der(seg_sizes)?;
    Ok(ca_cert)
}

fn merge_ca_cert_der(seg_sizes: Vec<i32>) -> anyhow::Result<Vec<u8>> {
    let mut ca_cert: Vec<u8> = vec![];

    for i in 0..seg_sizes.len() {
        let filename = format!("ca_cert_part{:}.bin", i);
        let args:Vec<String> = vec![
            "--hierarchy".to_string(), "owner".to_string(),
            "--size".to_string(), format!("{:}",seg_sizes[i]), 
            "--output".to_string(), filename.clone(),
            format!("{:}{:}", INDEX_PREFIX, i)];

        let output = run_tpm2_nvread(Some(args))?;
        if !output.status.success() {
            println!("Err: {:?}", String::from_utf8(output.stderr));
            bail!("Retrieve CA Certificate Part {:} Failed!", i)
        }
        let mut ca_part = fs::read(filename.clone())?;
        ca_cert.append(&mut ca_part);
        fs::remove_file(filename)?;
    }

    Ok(ca_cert)
}

pub fn retrieve_ek_cert_der(cert_meta: &CertMeta) -> anyhow::Result<Vec<u8>> {    
    let filename = "ek_cert.bin".to_string();
    let args:Vec<String> = vec![
        "--hierarchy".to_string(), "owner".to_string(),
        "--size".to_string(), format!("{:}", cert_meta.seg_sizes[0]), 
        "--output".to_string(), filename.clone(),
        format!("{:}", EK_INDEX)];
    let output = run_tpm2_nvread(Some(args))?;
    if !output.status.success() {
        println!("Err: {:?}", String::from_utf8(output.stderr));
        bail!("Retrieve EK Certificate Failed!")
    }
    let ek_cert = fs::read(filename.clone())?;
    
    fs::remove_file(filename)?;
    Ok(ek_cert)
}

pub fn retrieve_raw_public_key(pubkey: &SubjectPublicKeyInfo) -> anyhow::Result<Vec<u8>> {
    match pubkey.parsed() {
        std::result::Result::Ok(PublicKey::EC(ec)) => {
            anyhow::Ok(ec.data().to_vec())
        }
        Err(e) => {
            bail!("CA public key parsed failed: {:} !", e);
        }
        _ => {
            bail!("CA public key unsupported type!");
        }
    }
}