use anyhow::{Ok, Result};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AmberNonceRequest {

}

#[derive(Serialize, Deserialize, Debug)]
pub struct AmberNonceResponse {
    
}

pub fn get_nonce(req: AmberNonceRequest) -> Result<AmberNonceResponse> {
    Ok(AmberNonceResponse{})
}