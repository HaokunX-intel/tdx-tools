use anyhow::Result;
use efivar::efi::VariableName;
use efivar::system;
use std::str::FromStr;

const DEFAULT_KBS_URL_FILE: &str = "KBSURL-0d9b4a60-e0bf-4a66-b9b1-db1b98f87770";
const DEFAULT_KBS_CERT_FILE: &str = "KBSCert-d2bf05a0-f7f8-41b6-b0ff-ad1a31c34d37";
const DEFAULT_KBS_USER_TOKEN_FILE: &str = "KBSUserData-732284dd-70c4-472a-aa45-1ffda02caf74";

#[derive(Default, Debug)]
pub struct KBSParams {
    pub url: Vec<u8>,
    pub certification: Vec<u8>,
    pub user_token: Vec<u8>,
}

pub fn retrieve_kbs_params() -> Result<KBSParams> {
    let var_manager = system();

    let url_name =
        VariableName::from_str(DEFAULT_KBS_URL_FILE).expect("Failed to create variable name url");
    let mut url_bytes: Vec<u8> = vec![0u8; 4096];
    let (_url_data_size, _url_data_flags) = var_manager
        .read(&url_name, &mut url_bytes)
        .expect("Failed to read variable url");

    let cert_name =
        VariableName::from_str(DEFAULT_KBS_CERT_FILE).expect("Failed to create variable name cert");
    let mut cert_bytes: Vec<u8> = vec![0u8; 4096];
    let (_cert_data_size, _cert_data_flags) = var_manager
        .read(&cert_name, &mut cert_bytes)
        .expect("Failed to read variable cert");

    let user_token_name = VariableName::from_str(DEFAULT_KBS_USER_TOKEN_FILE)
        .expect("Failed to create variable name user token");

    let mut user_token_bytes: Vec<u8> = vec![0u8; 4096];
    let (_user_token_data_size, _user_token_data_flags) = var_manager
        .read(&user_token_name, &mut user_token_bytes)
        .expect("Failed to read variable token");

    Ok(KBSParams {
        url: url_bytes[0.._url_data_size].to_vec(),
        certification: cert_bytes[0.._cert_data_size].to_vec(),
        user_token: user_token_bytes[0.._user_token_data_size].to_vec(),
    })
}
