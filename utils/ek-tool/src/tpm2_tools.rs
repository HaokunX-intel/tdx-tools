use std::{process::Output, io};


use anyhow::Ok;

use crate::utils::run_command;

const TPMNVREAD_PUBLIC_CMD : &str= "tpm2_nvreadpublic";
const TPM2_NVREAD_CMD: &str="tpm2_nvread";
const TPM2_CREATE_AK_CMD: &str="tpm2_createak";
const TPM2_QUOTE_CMD: &str="tpm2_quote";
const TPM2_CHECKQUOTE_CMD: &str="tpm2_checkquote";
const TPM2_READ_PUBLIC_CMD: &str="tpm2_readpublic";

pub fn run_tpmnvread_public(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPMNVREAD_PUBLIC_CMD.to_string(), args)
}

pub fn parse_meta(raw_vec: Vec<u8>) -> anyhow::Result<Vec<i32>> {
    let mut sizes: Vec<i32> = vec![];
    
    // let mut size_cnt :i32= 0; 
    let raw_str: String = String::from_utf8(raw_vec)?;
    let lines: Vec<&str> = raw_str.split('\n').collect();
    for line in lines {
        if line.find("size").is_some() {
            // println!("{:}", line);
            // if size_cnt > 0 {
            let words: Vec<&str> = line.trim().split(' ').collect();
            // println!("{:?}", words);
            if words.len() > 1 {
                let s: i32 = words[1].to_string().parse()?;
                sizes.push(s);
            }
            // }
            // size_cnt += 1;
        }
    }

    // for size in sizes  {
    //     println!("{:}", size);
    // }
    Ok(sizes)
}

pub fn run_tpm2_nvread(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPM2_NVREAD_CMD.to_string(), args)
}

pub fn run_tpm2_create_ak(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPM2_CREATE_AK_CMD.to_string(), args)
}

pub fn run_tpm2_quote(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPM2_QUOTE_CMD.to_string(), args)
}

pub fn run_tpm2_checkquote(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPM2_CHECKQUOTE_CMD.to_string(), args)
}

pub fn run_tpm2_read_public(args: Option<Vec<String>>) -> io::Result<Output> {
    run_command(TPM2_READ_PUBLIC_CMD.to_string(), args)
}