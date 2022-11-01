mod client;
mod settings;

use crate::settings::Settings;
use clap::{Parser, Subcommand};
use helium_config_service_cli::{DevaddrRange, Eui, PrettyJson, Result, Route};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "helium-config-cli")]
#[command(author, version, about = "CLI for helium packet router config service", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file
    #[arg(short, long, default_value = "./config/default.toml")]
    config: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Commands {
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
}

#[derive(Debug, Subcommand)]
enum OrgCommands {
    /// List Orgs
    List,

    /// View your Org (oui taken from default.toml)
    Get,
}

#[derive(Debug, Subcommand)]
enum RouteCommands {
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let settings = Settings::new(&cli.config)?;
    fs::create_dir_all(&settings.out_dir)?;

    match cli.command {
        Commands::Init => Settings::interactive_init()?,
        Commands::Devaddr {
            start_addr,
            end_addr,
            route,
            commit,
        } => {
            let devaddr = DevaddrRange::new(&start_addr, &end_addr)?;

            match route {
                Some(route_id) => {
                    let mut r = Route::from_file(&settings.out_dir, route_id.clone())?;
                    r.add_devaddr(devaddr);
                    if commit {
                        println!("Devaddr added");
                        r.write(&settings.out_dir)?;
                    } else {
                        println!("Replace {route_id} with the following, or pass --commit:");
                        r.print_pretty_json()?;
                    }
                }
                None => {
                    println!("Put this into the 'devaddr_ranges' section of your file:");
                    devaddr.print_pretty_json()?;
                }
            }
        }
        Commands::Eui {
            dev_eui,
            app_eui,
            route,
            commit,
        } => {
            let eui = Eui::new(&app_eui, &dev_eui)?;

            match route {
                Some(route_id) => {
                    let mut r = Route::from_file(&settings.out_dir, route_id.clone())?;
                    r.add_eui(eui);

                    if commit {
                        println!("EUI added");
                        r.write(&settings.out_dir)?;
                    } else {
                        println!("Replace {route_id} with the following, or pass --commit:");
                        r.print_pretty_json()?;
                    }
                }
                None => {
                    println!("Put this into the 'euis' section of your file:");
                    eui.print_pretty_json()?;
                }
            }
        }
        Commands::Org { command } => {
            let mut org_client = client::OrgClient::new(&settings.config_host).await?;
            match command {
                OrgCommands::List => org_client.list().await?.print_pretty_json()?,
                OrgCommands::Get => org_client.get(settings.oui).await?.print_pretty_json()?,
            };
        }
        Commands::Route { command } => {
            let mut route_client = client::RouteClient::new(&settings.config_host).await?;
            match command {
                RouteCommands::List { commit } => {
                    let response = route_client.list(settings.oui, settings.owner).await?;
                    response.print_pretty_json()?;

                    if commit {
                        response.write_all(&settings.out_dir)?;
                    }
                }
                RouteCommands::Get { id, commit } => {
                    let response = route_client.get(id, settings.owner).await?;
                    response.print_pretty_json()?;

                    if commit {
                        response.write(&settings.out_dir)?;
                    }
                }
                RouteCommands::Create { commit } => match commit {
                    false => {
                        println!("Doing nothing. Pass the --commit flag to create a route in the config service");
                    }
                    true => {
                        let response = route_client
                            .create(
                                settings.net_id,
                                settings.oui,
                                settings.max_copies,
                                settings.owner,
                            )
                            .await?;
                        response.print_pretty_json()?;
                        response.write(&settings.out_dir)?;
                    }
                },
                RouteCommands::Delete { id, commit } => {
                    let route = Route::from_file(&settings.out_dir, id.clone())?;
                    match commit {
                        false => {
                            println!("==============: DRY RUN :==============");
                            route.print_pretty_json()?;
                        }
                        true => {
                            let removed = route_client.delete(id, settings.owner).await.and_then(
                                |route| {
                                    println!("==============: DELETED :==============");
                                    route.remove(&settings.out_dir)?;
                                    Ok(route)
                                },
                            )?;
                            removed.print_pretty_json()?;
                        }
                    }
                }
                RouteCommands::Push { id, commit } => {
                    let route = Route::from_file(&settings.out_dir, id.clone())?;
                    match commit {
                        false => {
                            println!("==============: DRY RUN :==============");
                            route.print_pretty_json()?;
                        }
                        true => {
                            let updated = route_client.push(route, settings.owner).await.and_then(
                                |updated_route| {
                                    println!("==============: PUSHED :==============");
                                    updated_route.write(&settings.out_dir)?;
                                    Ok(updated_route)
                                },
                            )?;
                            updated.print_pretty_json()?;
                        }
                    }
                }
            }
        }
    };

    Ok(())
}
