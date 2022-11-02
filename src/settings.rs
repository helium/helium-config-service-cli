use config::{Config, File};
use dialoguer::{Confirm, Input};
use helium_config_service_cli::{HexField, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Deserialize, Serialize)]
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
        if !path.exists() {
            println!("{path:?} does not exist, let's create it...");
            Self::interactive_init(path)?;
        }
        Config::builder()
            .add_source(File::with_name(path.to_str().expect("settings file name")))
            .build()
            .and_then(|config| config.try_deserialize())
            .map_err(|e| e.into())
    }

    pub fn interactive_init(path: &Path) -> Result<()> {
        let oui = Input::new().with_prompt("Assigned OUI").interact()?;
        let net_id = Input::<String>::new()
            .with_prompt("Net ID")
            .validate_with(|input: &String| -> std::result::Result<(), &str> {
                match HexField::<6>::from_str(input) {
                    Ok(_) => Ok(()),
                    Err(_err) => Err("insert a hex number with 6 digits"),
                }
            })
            .interact()?;
        let owner = Input::new().with_prompt("Owner Public Key").interact()?;
        let config_host = Input::new()
            .with_prompt("Config Service Host")
            .default("http://localhost:50051".into())
            .interact()?;
        let out_dir: PathBuf = Input::<String>::new()
            .with_prompt("Route Directory")
            .default("./routes".into())
            .interact()?
            .into();
        let max_copies = Input::new()
            .with_prompt("Default Max Copies")
            .default(15)
            .interact()?;

        let s = Settings {
            oui,
            net_id: HexField::<6>::from_str(&net_id)?,
            owner,
            config_host,
            out_dir,
            max_copies,
        };
        s.maybe_write(path)
    }

    pub fn maybe_write(&self, path: &Path) -> Result<()> {
        let output = toml::to_string_pretty(self)?;
        println!("\n======== Configuration ==========");
        println!("{output}");
        if Confirm::new()
            .with_prompt(format!("Write to file {}?", path.display()))
            .interact()?
        {
            self.write(path)?;
        }
        Ok(())
    }

    pub fn set_oui(self, oui: u64) -> Self {
        Self { oui, ..self }
    }

    pub fn filename(&self, dir: &Path) -> PathBuf {
        dir.join(format!("oui-{}.toml", self.oui))
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let output = toml::to_string_pretty(self)?;
        fs::write(path, &output)?;
        Ok(())
    }
}
