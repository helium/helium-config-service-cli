use crate::{hex_field, region::Region, server::FlowType};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "helium-config-cli")]
#[command(author, version, about = "CLI for helium packet router config service", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Config file
    #[arg(short, long, default_value = "./config/default.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize your settings file
    Init,
    /// Information about your identity
    Info,
    /// Make a new keypair
    Generate {
        #[arg(short, long)]
        commit: bool,
    },
    /// Org commands
    Org {
        #[command(subcommand)]
        command: OrgCommands,
    },

    /// Route commands
    Route {
        #[command(subcommand)]
        command: RouteCommands,
    },

    /// Output format for inserting Devaddr
    Devaddr {
        #[arg(value_parser = hex_field::validate_devaddr)]
        start_addr: hex_field::HexDevAddr,
        #[arg(value_parser = hex_field::validate_devaddr)]
        end_addr: hex_field::HexDevAddr,
        /// ID of route to apply devaddr range to (same as filename)
        route: Option<String>,
        /// Add the verified devaddr entry to the routes file
        #[arg(short, long)]
        commit: bool,
    },
    /// Output format for inserting EUI
    Eui {
        #[arg(short, long, value_parser = hex_field::validate_eui)]
        dev_eui: hex_field::HexEui,
        #[arg(short, long, value_parser = hex_field::validate_eui)]
        app_eui: hex_field::HexEui,
        /// ID of route to apply eui to (same as filename)
        route: Option<String>,
        /// Add the verified eui entry to the routes file
        #[arg(short, long)]
        commit: bool,
    },
    /// Output format for inserting Protocol
    Protocol {
        /// Protocol to route packets over
        #[arg(value_enum)]
        protocol: ProtocolType,
        #[arg(long, default_value = "localhost")]
        host: String,
        #[arg(long, default_value = "8080")]
        port: u32,
        /// ID of route to apply protocol (same as filename)
        route: Option<String>,
        /// Write the protocol into the route file
        #[arg(long)]
        commit: bool,
    },
    /// Map a LoRa region to a Port
    GwmpMapping {
        #[arg(value_enum)]
        region: Region,
        port: u32,
        /// ID of the route to apply the mapping
        route: Option<String>,
        /// Write the protocol into the route file
        #[arg(long)]
        commit: bool,
    },
    /// Update the details of an Http Route
    Http {
        #[arg(short, long, value_enum)]
        flow_type: FlowType,
        #[arg(short, long)]
        dedupe_timeout: u32,
        #[arg(short, long)]
        path: String,
        /// ID of the route to apply the mapping
        route: Option<String>,
        /// Write the protocol into the route file
        #[arg(long)]
        commit: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ProtocolType {
    PacketRouter,
    Gwmp,
    Http,
}

#[derive(Debug, Subcommand)]
pub enum OrgCommands {
    /// List Orgs
    List,

    /// View your Org (oui taken from default.toml)
    Get,

    /// Create an OUI Organization under the Helium NetID
    CreateHelium(OrgCreateHelium),
    /// Create a Roaming Organization
    CreateRoamer(OrgCreateRoamer),
}

#[derive(Debug, Subcommand)]
pub enum RouteCommands {
    /// List your Routes
    List {
        /// Write all routes to their own files locally
        #[arg(short, long)]
        commit: bool,
    },
    /// Get a Route
    Get {
        id: String,
        /// Create or overwrite existing route with <ID>
        #[arg(short, long)]
        commit: bool,
    },
    /// Create a Route
    Create {
        /// Route will not be created without this flag
        #[arg(short, long)]
        commit: bool,
    },
    /// Delete a Route
    Delete {
        id: String,
        /// Are you sure?
        #[arg(short, long)]
        commit: bool,
    },
    /// Push an updated route (note: nonce will auto-increment)
    Push {
        id: String,
        /// Are you sure?
        #[arg(short, long)]
        commit: bool,
    },
}

#[derive(Debug, Args)]
pub struct OrgCreateHelium {
    #[arg(long)]
    pub owner: String,
    #[arg(long)]
    pub payer: String,
    #[arg(long)]
    pub devaddr_count: u64,
    #[arg(long)]
    pub commit: bool,
}

#[derive(Debug, Args)]
pub struct OrgCreateRoamer {
    #[arg(long)]
    pub owner: String,
    #[arg(long)]
    pub payer: String,
    #[arg(long)]
    pub net_id: u64,
    #[arg(long)]
    pub commit: bool,
}
