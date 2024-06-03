use serde::Deserialize;
use std::path::Path;

use super::prelude::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub data_dir: String,
    pub postgre_url: String,
    pub redis_url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let profile = if cfg!(debug_assertions) {
            "dev"
        } else {
            "prod"
        };

        let filename = format!(".env.{profile}.local");
        if Path::new(&filename).exists() {
            dotenv::from_filename(filename).ok();
        }
        dotenv::dotenv().ok();

        Ok(envy::from_env::<Self>()?)
    }
}
