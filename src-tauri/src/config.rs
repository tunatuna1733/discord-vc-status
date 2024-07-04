use confy;
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
