use clap::Parser;
use dialoguer::Input;
use helium_config_service_cli::{
    client,
    cmds::{
        AddCommands, AddDevaddr, AddEui, AddGwmpMapping, AddHttpSettings, AddProtocol, Cli,
        Commands, CreateHelium, CreateRoaming, CreateRoute, GenerateKeypair, GenerateRoute, GetOrg,
        GetRoute, PathBufKeypair, ProtocolType, UpdateRoute, ENV_CONFIG_HOST, ENV_KEYPAIR_BIN,
        ENV_MAX_COPIES, ENV_NET_ID, ENV_OUI,
    },
    hex_field::{self},
    route::Route,
    server::{Protocol, Server},
    DevaddrRange, Eui, PrettyJson, Result,
};
use rand::rngs::OsRng;
use serde::Serialize;
use std::{fmt::Display, fs};

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();

    handle_cli(cli).await?;

    Ok(())
}

#[derive(Debug, Serialize)]
enum Msg {
    Success(String),
    Error(String),
}

impl Msg {
    fn ok(msg: String) -> Result<Self> {
        Ok(Self::Success(msg))
    }
    fn err(msg: String) -> Result<Self> {
        Ok(Self::Error(msg))
    }
}

impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::Success(msg) => write!(f, "\u{2713} {}", msg),
            Msg::Error(msg) => write!(f, "\u{2717} {}", msg),
        }
    }
}

async fn handle_cli(cli: Cli) -> Result {
    let msg: Msg = match cli.command {
        Commands::EnvInit => env_init().await,
        // File Creation
        Commands::GenerateKeypair(args) => generate_keypair(args),
        Commands::GenerateRoute(args) => generate_route(args),
        // API Commands
        Commands::GetRoute(args) => get_route(args).await,
        Commands::GetOrg(args) => get_org(args).await,
        Commands::CreateRoute(args) => create_route(args).await,
        Commands::CreateHelium(args) => create_helium_org(args).await,
        Commands::CreateRoaming(args) => create_roaming_org(args).await,
        Commands::UpdateRoute(args) => update_route(args).await,
        // File updating commands
        Commands::Add { command } => match command {
            AddCommands::Devaddr(args) => add_devaddr(args).await,
            AddCommands::Eui(args) => add_eui(args).await,
            AddCommands::Protocol(args) => add_protocol(args).await,
            AddCommands::GwmpMapping(args) => add_gwmp_mapping(args).await,
            AddCommands::Http(args) => add_http_settings(args).await,
        },
    }?;

    println!("{msg}");
    Ok(())
}

async fn env_init() -> Result<Msg> {
    println!("----- Leave blank to ignore...");
    let config_host: String = Input::new()
        .with_prompt("Config Service Host")
        .allow_empty(true)
        .interact()?;
    let keypair_path: String = Input::<String>::new()
        .with_prompt("Keypair Location")
        .with_initial_text("./keypair.bin")
        .allow_empty(true)
        .interact()?
        .into();
    println!("----- Enter all zeros to ignore...");
    let net_id = Input::<hex_field::HexNetID>::new()
        .with_prompt("Net ID")
        .with_initial_text("000000")
        .interact()?;
    println!("----- Enter zero to ignore...");
    let oui: u64 = Input::new()
        .with_prompt("Assigned OUI")
        .with_initial_text("0")
        .allow_empty(true)
        .interact()?;
    let max_copies: u32 = Input::new()
        .with_prompt("Default Max Copies")
        .allow_empty(true)
        .with_initial_text("15")
        .interact()?;

    let mut report = vec![
        "".to_string(),
        "Put these in your environment".to_string(),
        "------------------------------------".to_string(),
    ];
    if !config_host.is_empty() {
        report.push(format!("{ENV_CONFIG_HOST}={config_host}"));
    }
    if !keypair_path.is_empty() {
        report.push(format!("{ENV_KEYPAIR_BIN}={keypair_path}"))
    }
    if net_id != hex_field::net_id(0) {
        report.push(format!("{ENV_NET_ID}={net_id}"));
    }
    if oui != 0 {
        report.push(format!("{ENV_OUI}={oui}"));
    }
    if max_copies != 0 {
        report.push(format!("{ENV_MAX_COPIES}={max_copies}"));
    }

    Msg::ok(report.join("\n"))
}

async fn add_devaddr(args: AddDevaddr) -> Result<Msg> {
    let devaddr = DevaddrRange::new(args.start_addr, args.end_addr)?;
    if !args.commit {
        return Msg::ok(format!(
            "valid range, insert into `devaddr_ranges` section\n{}",
            devaddr.pretty_json()?
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.add_devaddr(devaddr);
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}

async fn add_eui(args: AddEui) -> Result<Msg> {
    let eui = Eui::new(args.app_eui, args.dev_eui)?;
    if !args.commit {
        return Msg::ok(format!(
            "valid eui, insert into `euis` section\n{}",
            eui.pretty_json()?
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.add_eui(eui);
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}

async fn add_protocol(args: AddProtocol) -> Result<Msg> {
    let protocol = match args.protocol {
        ProtocolType::PacketRouter => Protocol::default_packet_router(),
        ProtocolType::Gwmp => Protocol::default_gwmp(),
        ProtocolType::Http => Protocol::default_http(),
    };
    let server = Server::new(args.host, args.port, protocol);
    if !args.commit {
        return Msg::ok(format!(
            "valid protocol, insert into `server` section\n{}",
            server.pretty_json()?
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}

async fn add_gwmp_mapping(args: AddGwmpMapping) -> Result<Msg> {
    let mapping = Protocol::make_gwmp_mapping(args.region, args.port);

    if !args.commit {
        return Msg::ok(format!(
            "valid mapping, insert into `mapping` section\n{}",
            mapping.pretty_json()?
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.gwmp_add_mapping(mapping)?;
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}

async fn add_http_settings(args: AddHttpSettings) -> Result<Msg> {
    let http = Protocol::make_http(args.flow_type, args.dedupe_timeout, args.path);

    if !args.commit {
        return Msg::ok(format!("valid http settings\n{}", http.pretty_json()?));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.http_update(http)?;
    route.write(&args.route_file)?;
    Msg::ok(format!("{} written", args.route_file.display()))
}

fn generate_keypair(args: GenerateKeypair) -> Result<Msg> {
    let key = helium_crypto::Keypair::generate(
        helium_crypto::KeyTag {
            network: helium_crypto::Network::MainNet,
            key_type: helium_crypto::KeyType::Ed25519,
        },
        &mut OsRng,
    );
    if let Some(parent) = args.out_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.out_file, &key.to_vec())?;
    Msg::ok(format!(
        "New Keypair created and written to {:?}",
        args.out_file.display()
    ))
}

fn generate_route(args: GenerateRoute) -> Result<Msg> {
    if args.out_file.exists() && !args.commit {
        return Msg::err(format!(
            "{} exists, pass `--commit` to override",
            args.out_file.display()
        ));
    }

    let route = Route::new(args.net_id, args.oui, args.max_copies);
    route.write(&args.out_file)?;

    Msg::ok(format!("{} created", args.out_file.display()))
}

async fn get_route(args: GetRoute) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route = client
        .get(&args.route_id, &args.owner, &args.keypair.to_keypair()?)
        .await?;

    if args.commit {
        route.write(&args.route_out_dir)?;
        return Msg::ok(format!(
            "{}/{} written",
            &args.route_out_dir.display(),
            route.filename()
        ));
    }
    Msg::ok(route.pretty_json()?)
}

async fn get_org(args: GetOrg) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host).await?;
    let org = client.get(args.oui).await?;

    Msg::ok(org.pretty_json()?)
}

async fn create_route(args: CreateRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let created_route = client
            .create(
                route.net_id,
                route.oui,
                route.max_copies,
                &args.owner,
                args.keypair.to_keypair()?,
            )
            .await?;
        created_route.write(&args.route_out_dir)?;

        return Msg::ok(format!(
            "{}/{} written",
            &args.route_out_dir.display(),
            created_route.filename()
        ));
    }
    Msg::ok(format!(
        "{} is valid, pass `--commit` to create",
        &args.route_file.display()
    ))
}

async fn update_route(args: UpdateRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let updated_route = client
            .push(route, &args.owner, args.keypair.to_keypair()?)
            .await?;
        updated_route.write(args.route_file.as_path())?;
        return Msg::ok(format!("{} written", &args.route_file.display()));
    }
    Msg::ok(format!(
        "{} is valid, pass `--commit` to update",
        &args.route_file.display()
    ))
}

async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host).await?;
        let org = client
            .create_helium(
                &args.owner,
                &args.payer,
                args.devaddr_count,
                args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(format!(
            "Helium Organization Created: \n{}",
            org.pretty_json()?
        ));
    }
    Msg::ok("pass `--commit` to create Helium organization".to_string())
}

async fn create_roaming_org(args: CreateRoaming) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host).await?;
        let org = client
            .create_roamer(
                &args.owner,
                &args.payer,
                args.net_id.into(),
                args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(format!(
            "Roaming Organization Created: \n{}",
            org.pretty_json()?
        ));
    }
    Msg::ok("pass `--commit` to create Roaming organization".to_string())
}
