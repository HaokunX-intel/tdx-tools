use std::fs;
use anyhow::{Ok, Result, bail};

const ACPI_FILE: &str = "/sys/firmware/acpi/tables/CCEL";
const CCEL_FILE: &str = "/sys/firmware/acpi/tables/data/CCEL";

pub fn retrieve_event_logs() -> Result<Vec<u8>> {
    // the metadata of event logs
    let meta = fs::read(ACPI_FILE)?;
    let length: usize = get_u64(&meta, 40)? as usize;

    // the data of event logs
    let data = fs::read(CCEL_FILE)?;
    if data.len() != length {
        bail!("Invalid Event logs: length != {}", length)
    }
    Ok(data)
}

fn get_u64(raw: &Vec<u8>, addr: usize) -> Result<u64> {
    if addr + 8 > raw.len() {
        bail!("Parse u64 Failed!")
    }
    let var_bytes:[u8;8] = raw[addr..addr+8].try_into()?;
    Ok(u64::from_le_bytes(var_bytes))
}
