use config::{Config, File};
use helium_config_service_cli::{HexField, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub oui: u64,
    #[serde(deserialize_with = "HexField::<6>::deserialize")]
    pub net_id: HexField<6>,
    pub owner: String,
    pub config_host: String,
    pub out_dir: PathBuf,
    pub max_copies: u32,
}

impl Settings {
    pub fn new(path: &Path) -> Result<Self> {
        Config::builder()
            .add_source(File::with_name(path.to_str().expect("settings file name")))
            .build()
            .and_then(|config| config.try_deserialize())
            .map_err(|e| e.into())
    }
}
