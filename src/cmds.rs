use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use clap::{Parser, Subcommand};

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
        start_addr: String,
        end_addr: String,
        /// ID of route to apply devaddr range to (same as filename)
        route: Option<String>,
        /// Add the verified devaddr entry to the routes file
        #[arg(short, long)]
        commit: bool,
    },
    /// Output format for inserting EUI
    Eui {
        #[arg(short, long)]
        dev_eui: String,
        #[arg(short, long)]
        app_eui: String,
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

    /// Create an organization
    Create {
        #[arg(long)]
        oui: u64,
        #[arg(long)]
        commit: bool,
    },
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
