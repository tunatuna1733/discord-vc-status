use confy;
use keyring::Entry;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub refresh_token: String,
}

pub fn get_config() -> Result<Config, confy::ConfyError> {
    match confy::load("discord-vc-status", "discord-vc-status") {
        Ok(r) => Ok(r),
        Err(e) => Err(e),
    }
}

pub fn set_config(config: Config) -> Result<(), confy::ConfyError> {
    match confy::store("discord-vc-status", "discord-vc-status", config) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn save_refresh_token(refresh_token: String) -> Result<(), keyring::Error> {
    let entry = match Entry::new("discord-vc-status", "refresh_token") {
        Ok(e) => e,
        Err(err) => {
            return Err(err);
        }
    };
    if let Err(err) = entry.set_password(&refresh_token) {
        return Err(err);
    }
    Ok(())
}

pub fn get_refresh_token() -> Result<String, keyring::Error> {
    let entry = match Entry::new("discord-vc-status", "refresh_token") {
        Ok(e) => e,
        Err(err) => {
            return Err(err);
        }
    };
    let token = match entry.get_password() {
        Ok(t) => t,
        Err(err) => {
            return Err(err);
        }
    };
    Ok(token)
}
