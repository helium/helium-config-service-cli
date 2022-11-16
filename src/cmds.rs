use crate::{
    hex_field::{self, HexNetID},
    region::Region,
    server::FlowType,
    Result,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use helium_crypto::PublicKey;
use std::path::PathBuf;

pub const ENV_CONFIG_HOST: &str = "HELIUM_CONFIG_HOST";
pub const ENV_KEYPAIR_BIN: &str = "HELIUM_KEYPAIR_BIN";
pub const ENV_NET_ID: &str = "HELIUM_NET_ID";
pub const ENV_OUI: &str = "HELIUM_OUI";
pub const ENV_MAX_COPIES: &str = "HELIUM_MAX_COPIES";

#[derive(Debug, Parser)]
#[command(name = "helium-config-cli-alt")]
#[command(author, version, about, long_about=None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        global = true,
        long,
        env = ENV_CONFIG_HOST,
        default_value = "http://50.18.149.124:50051"
    )]
    pub config_host: String,

    #[arg(
        global = true,
        long,
        env = ENV_KEYPAIR_BIN,
        default_value = "./keypair.bin"
    )]
    pub keypair: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Make Environment variables to ease repeated use
    EnvInit,
    /// View information about your environment
    EnvInfo,
    /// Make a new keypair
    GenerateKeypair(GenerateKeypair),
    /// Make an empty route file edit
    GenerateRoute(GenerateRoute),

    /// Get all Routes for an OUI
    GetRoutes(GetRoutes),
    /// Get a Route by ID and write to file
    GetRoute(GetRoute),
    /// Get an Organization you own
    GetOrg(GetOrg),

    /// Create a Route from a file
    CreateRoute(CreateRoute),
    /// Create a new Helium Organization
    CreateHelium(CreateHelium),
    /// Create a new Roaming Organization (admin only)
    CreateRoaming(CreateRoaming),

    /// Update a Route
    UpdateRoute(UpdateRoute),

    /// Updating sections in Routes
    Add {
        #[command(subcommand)]
        command: AddCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddCommands {
    /// Add Devaddr Range to route file (default: ./new_route.json)
    Devaddr(AddDevaddr),
    /// Add EUI to route file (default: ./new_route.json)
    Eui(AddEui),
    /// Add Protocol to route file (default: ./new_route.json)
    Protocol(AddProtocol),

    // Protocol Specific commands
    //
    /// Map a LoRa regiont to a Port
    GwmpMapping(AddGwmpMapping),
    /// Update the details of an Http Route
    Http(AddHttpSettings),
}

#[derive(Debug, Args)]
pub struct AddGwmpMapping {
    #[arg(value_enum)]
    pub region: Region,
    pub port: u32,
    /// Path of route to apply gwmp mapping to
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,
    /// Write the protocol into the route file
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct AddHttpSettings {
    #[arg(short, long, value_enum)]
    pub flow_type: FlowType,
    #[arg(short, long, default_value = "250")]
    pub dedupe_timeout: u32,
    /// Just the path part of the Server URL
    ///
    /// The rest will be taken from the Server {host}:{port}
    #[arg(short, long)]
    pub path: String,
    /// Path of route to apply http settings to
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,
    /// Write the protocol into the route file
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct GenerateKeypair {
    #[arg(default_value = "./keypair.bin")]
    pub out_file: PathBuf,

    /// overwrite <out_file> if it already exists
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct GenerateRoute {
    #[arg(long, env = ENV_NET_ID, default_value = "C00053")]
    pub net_id: HexNetID,
    #[arg(long, env = ENV_OUI)]
    pub oui: u64,
    #[arg(long, env = ENV_MAX_COPIES, default_value = "5")]
    pub max_copies: u32,

    #[arg(long, default_value = "./new_route.json")]
    pub out_file: PathBuf,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct GetRoutes {
    #[arg(long, env = ENV_OUI)]
    pub oui: u64,
    #[arg(short, long)]
    pub owner: PublicKey,
    #[arg(from_global)]
    pub keypair: PathBuf,
    // #[arg(long, default_value = "./routes")]
    // pub route_out_dir: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long, default_value = "./routes")]
    pub route_out_dir: PathBuf,
    /// Write all routes --route_out_dir
    ///
    /// WARNING!!! This will overwrite unupdated routes
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct GetRoute {
    #[arg(short, long)]
    pub route_id: String,
    #[arg(short, long)]
    pub owner: PublicKey,
    #[arg(from_global)]
    pub keypair: PathBuf,
    #[arg(long, default_value = "./routes")]
    pub route_out_dir: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct GetOrg {
    #[arg(long, env = "HELIUM_OUI")]
    pub oui: u64,
    #[arg(from_global)]
    pub config_host: String,
}

#[derive(Debug, Args)]
pub struct CreateRoute {
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,
    #[arg(long)]
    pub owner: PublicKey,
    #[arg(from_global)]
    pub keypair: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long, default_value = "./routes")]
    pub route_out_dir: PathBuf,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct UpdateRoute {
    #[arg(long)]
    pub route_file: PathBuf,
    #[arg(long)]
    pub owner: PublicKey,
    #[arg(from_global)]
    pub keypair: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct CreateHelium {
    #[arg(long)]
    pub owner: PublicKey,
    #[arg(long)]
    pub payer: PublicKey,
    #[arg(long)]
    pub devaddr_count: u64,
    #[arg(from_global)]
    pub keypair: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct CreateRoaming {
    #[arg(long)]
    pub owner: PublicKey,
    #[arg(long)]
    pub payer: PublicKey,
    #[arg(long)]
    pub net_id: HexNetID,
    #[arg(from_global)]
    pub keypair: PathBuf,
    #[arg(from_global)]
    pub config_host: String,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct AddDevaddr {
    #[arg(value_parser = hex_field::validate_devaddr)]
    pub start_addr: hex_field::HexDevAddr,
    #[arg(value_parser = hex_field::validate_devaddr)]
    pub end_addr: hex_field::HexDevAddr,

    /// Path of route to apply devaddr range to
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,

    /// Add the verified devaddr entry to the routes file
    #[arg(short, long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct AddEui {
    #[arg(short, long, value_parser = hex_field::validate_eui)]
    pub dev_eui: hex_field::HexEui,
    #[arg(short, long, value_parser = hex_field::validate_eui)]
    pub app_eui: hex_field::HexEui,
    /// Path of route to apply devaddr range to
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,
    /// Add the verified eui entry to the routes file
    #[arg(short, long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct AddProtocol {
    /// Protocol to route packets over
    #[arg(value_enum)]
    pub protocol: ProtocolType,
    #[arg(long, default_value = "localhost")]
    pub host: String,
    #[arg(long, default_value = "8080")]
    pub port: u32,
    /// Path of route to apply devaddr range to
    #[arg(long, default_value = "./new_route.json")]
    pub route_file: PathBuf,
    /// Add the verified eui entry to the routes file
    #[arg(short, long)]
    pub commit: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ProtocolType {
    PacketRouter,
    Gwmp,
    Http,
}

pub trait PathBufKeypair {
    fn to_keypair(&self) -> Result<helium_crypto::Keypair>;
}

impl PathBufKeypair for PathBuf {
    fn to_keypair(&self) -> Result<helium_crypto::Keypair> {
        let data = std::fs::read(&self)?;
        Ok(helium_crypto::Keypair::try_from(&data[..])?)
    }
}
