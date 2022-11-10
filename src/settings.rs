use anyhow::anyhow;
use config::{Config, File};
use dialoguer::{Confirm, Input};
use helium_config_service_cli::hex_field;
use helium_config_service_cli::Result;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    /// Which OUI is being used
    pub oui: u64,
    /// NetID assigned to the OUI, or the Helium NetID
    pub net_id: hex_field::HexNetID,
    /// Public Key of the OUI owner
    pub owner: String,
    /// URI for the configuration service
    pub config_host: String,
    /// Destination for routing files
    pub out_dir: PathBuf,
    /// Default max_copies setting
    pub max_copies: u32,
    /// File to load keypair from
    pub keypair: PathBuf,
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

    pub fn interactive_init(path: &Path) -> Result {
        let oui = Input::new().with_prompt("Assigned OUI").interact()?;
        let net_id = Input::<hex_field::HexNetID>::new()
            .with_prompt("Net ID")
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
        let keypair: PathBuf = Input::<String>::new()
            .with_prompt("Where is your keypair?")
            .default("./keypair.bin".into())
            .interact()?
            .into();

        let s = Settings {
            oui,
            net_id,
            owner,
            config_host,
            out_dir,
            max_copies,
            keypair,
        };
        s.maybe_write(path)
    }

    pub fn maybe_write(&self, path: &Path) -> Result {
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

    pub fn write(&self, path: &Path) -> Result {
        let output = toml::to_string_pretty(self)?;
        fs::write(path, &output)?;
        Ok(())
    }

    pub fn keypair(&self) -> Result<helium_crypto::Keypair> {
        let data = std::fs::read(&self.keypair)?;
        Ok(helium_crypto::Keypair::try_from(&data[..])?)
    }

    pub fn maybe_generate_keypair(&self, commit: bool) -> Result {
        if self.keypair.exists() && !commit {
            return Err(anyhow!(
                "{:?} exists, to overwrite with new keypair pass `--commit`",
                self.keypair
            ));
        }
        self.generate_keypair()
    }

    pub fn generate_keypair(&self) -> Result {
        let key = helium_crypto::Keypair::generate(
            helium_crypto::KeyTag {
                network: helium_crypto::Network::MainNet,
                key_type: helium_crypto::KeyType::Ed25519,
            },
            &mut OsRng,
        );
        if let Some(parent) = self.keypair.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.keypair, &key.to_vec()).map_err(|e| e.into())
    }
}
