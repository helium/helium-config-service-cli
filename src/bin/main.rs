use anyhow::Context;
use clap::Parser;
use helium_config_service_cli::{
    client,
    cmds::{Cli as Main, Commands, OrgCommands, ProtocolType, RouteCommands},
    route::Route,
    server::{GwmpMap, Http, Protocol, Server},
    settings::Settings,
    DevaddrRange, Eui, PrettyJson, Result,
};

use serde_json::json;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result {
    let cli = Main::parse();
    let settings = Settings::new(&cli.config).context("reading settings")?;
    fs::create_dir_all(&settings.out_dir)?;

    match cli.command {
        Commands::Init => Settings::interactive_init(&cli.config)?,
        Commands::Info => {
            let output = json!({
                "oui": settings.oui,
                "host": settings.config_host,
                "default_max_copies": settings.max_copies,
                "net_id": settings.net_id,
                "keypair_location": settings.keypair,
                "keypair_pubkey": settings.keypair()?.public_key(),
                "owner_pubkey": settings.owner
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Commands::Generate { commit } => settings.maybe_generate_keypair(commit)?,
        Commands::Devaddr {
            start_addr,
            end_addr,
            route,
            commit,
        } => {
            let devaddr = DevaddrRange::new(start_addr, end_addr)?;
            update_route_section(
                &settings.out_dir,
                route,
                commit,
                RouteUpdate::AddDevaddr(devaddr),
                "devaddr_ranges",
            )?;
        }
        Commands::Eui {
            dev_eui,
            app_eui,
            route,
            commit,
        } => {
            let eui = Eui::new(app_eui, dev_eui)?;
            update_route_section(
                &settings.out_dir,
                route,
                commit,
                RouteUpdate::AddEui(eui),
                "euis",
            )?;
        }
        Commands::Protocol {
            protocol: protocol_type,
            host,
            port,
            route,
            commit,
        } => {
            let protocol = match protocol_type {
                ProtocolType::PacketRouter => Protocol::default_packet_router(),
                ProtocolType::Gwmp => Protocol::default_gwmp(),
                ProtocolType::Http => Protocol::default_http(),
            };
            let server = Server::new(host, port, protocol);
            update_route_section(
                &settings.out_dir,
                route,
                commit,
                RouteUpdate::SetServer(server),
                "server",
            )?;
        }
        Commands::GwmpMapping {
            region,
            port,
            route,
            commit,
        } => {
            let mapping = Protocol::make_gwmp_mapping(region, port);
            update_route_section(
                &settings.out_dir,
                route,
                commit,
                RouteUpdate::AddGwmpMapping(mapping),
                "mapping",
            )?;
        }
        Commands::Http {
            flow_type,
            dedupe_timeout,
            path,
            route,
            commit,
        } => {
            let http = Protocol::make_http(flow_type, dedupe_timeout, path);
            update_route_section(
                &settings.out_dir,
                route,
                commit,
                RouteUpdate::UpdateHttp(http),
                "protocol",
            )?;
        }
        Commands::Org { command } => {
            let mut org_client = client::OrgClient::new(&settings.config_host).await?;
            match command {
                OrgCommands::List => org_client.list().await?.print_pretty_json()?,
                OrgCommands::Get => org_client.get(settings.oui).await?.print_pretty_json()?,
                OrgCommands::CreateHelium(args) => match args.commit {
                    false => println!("==============: DRY RUN :=============="),
                    true => {
                        let response = org_client
                            .create_helium(
                                &args.owner,
                                &args.payer,
                                args.devaddr_count,
                                settings.keypair()?,
                            )
                            .await?;
                        println!("==============: CREATED :==============");
                        response.print_pretty_json()?;
                    }
                },
                OrgCommands::CreateRoamer(args) => match args.commit {
                    false => println!("==============: DRY RUN :=============="),
                    true => {
                        let response = org_client
                            .create_roamer(
                                &args.owner,
                                &args.payer,
                                args.net_id,
                                settings.keypair()?,
                            )
                            .await?;
                        println!("==============: CREATED :==============");
                        response.print_pretty_json()?;
                    }
                },
            };
        }
        Commands::Route { command } => {
            let mut route_client = client::RouteClient::new(&settings.config_host).await?;
            match command {
                RouteCommands::List { commit } => {
                    let response = route_client
                        .list(settings.oui, &settings.owner, settings.keypair()?)
                        .await?;
                    response.print_pretty_json()?;

                    if commit {
                        response.write_all(&settings.out_dir)?;
                    }
                }
                RouteCommands::Get { id, commit } => {
                    let response = route_client
                        .get(&id, &settings.owner, &settings.keypair()?)
                        .await?;
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
                                &settings.owner,
                                settings.keypair()?,
                            )
                            .await?;
                        response.print_pretty_json()?;
                        response.write(&settings.out_dir)?;
                    }
                },
                RouteCommands::Delete { id, commit } => {
                    let route = Route::from_id(&settings.out_dir, &id)?;
                    match commit {
                        false => {
                            println!("==============: DRY RUN :==============");
                            route.print_pretty_json()?;
                        }
                        true => {
                            let removed = route_client
                                .delete(&id, &settings.owner, settings.keypair()?)
                                .await
                                .and_then(|route| {
                                    println!("==============: DELETED :==============");
                                    route.remove(&settings.out_dir)?;
                                    Ok(route)
                                })?;
                            removed.print_pretty_json()?;
                        }
                    }
                }
                RouteCommands::Push { id, commit } => {
                    let route = Route::from_id(&settings.out_dir, &id)?;
                    match commit {
                        false => {
                            println!("==============: DRY RUN :==============");
                            route.print_pretty_json()?;
                        }
                        true => {
                            let updated = route_client
                                .push(route, &settings.owner, settings.keypair()?)
                                .await
                                .and_then(|updated_route| {
                                    println!("==============: PUSHED :==============");
                                    updated_route.write(&settings.out_dir)?;
                                    Ok(updated_route)
                                })?;
                            updated.print_pretty_json()?;
                        }
                    }
                }
            }
        }
    };

    Ok(())
}

fn update_route_section(
    out_dir: &Path,
    route: Option<String>,
    commit: bool,
    action: RouteUpdate,
    section_name: &str,
) -> Result {
    match route {
        Some(route_id) => {
            let mut route = Route::from_id(out_dir, &route_id)?;
            match action {
                RouteUpdate::AddDevaddr(range) => route.add_devaddr(range),
                RouteUpdate::AddEui(eui) => route.add_eui(eui),
                RouteUpdate::SetServer(server) => route.set_server(server),
                RouteUpdate::AddGwmpMapping(map) => route.gwmp_add_mapping(map)?,
                RouteUpdate::UpdateHttp(http) => route.http_update(http)?,
            };

            if commit {
                println!("{route_id} updated");
                route.write(out_dir)?;
            } else {
                println!("Replace {route_id} with the following, or pass --commit:");
                route.print_pretty_json()?;
            }
        }
        None => {
            println!("Put this into the '{section_name}' section of your file:");
            action.print_pretty_json()?;
        }
    }
    Ok(())
}

enum RouteUpdate {
    AddDevaddr(DevaddrRange),
    AddEui(Eui),
    SetServer(Server),
    AddGwmpMapping(GwmpMap),
    UpdateHttp(Http),
}

impl RouteUpdate {
    fn print_pretty_json(&self) -> Result {
        match self {
            RouteUpdate::AddDevaddr(d) => d.print_pretty_json()?,
            RouteUpdate::AddEui(e) => e.print_pretty_json()?,
            RouteUpdate::SetServer(s) => s.print_pretty_json()?,
            RouteUpdate::AddGwmpMapping(map) => map.print_pretty_json()?,
            RouteUpdate::UpdateHttp(http) => http.print_pretty_json()?,
        }
        Ok(())
    }
}
