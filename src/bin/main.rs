use anyhow::Context;
use clap::Parser;
use dialoguer::Input;
use helium_config_service_cli::{
    client,
    cmds::{
        AddCommands, AddDevaddr, AddEui, AddGwmpMapping, AddGwmpSettings, AddHttpSettings,
        AddPacketRouterSettings, Cli, Commands, CreateHelium, CreateRoaming, CreateRoute, EnvInfo,
        GenerateKeypair, GenerateRoute, GetOrg, GetOrgs, GetRoute, GetRoutes, PathBufKeypair,
        ProtocolCommands, RemoveRoute, SubnetMask, UpdateRoute, ENV_CONFIG_HOST, ENV_KEYPAIR_BIN,
        ENV_MAX_COPIES, ENV_NET_ID, ENV_OUI,
    },
    hex_field,
    route::Route,
    server::{Protocol, Server},
    subnet::RouteSubnets,
    DevaddrRange, Eui, PrettyJson, Result,
};
use helium_crypto::Keypair;
use rand::rngs::OsRng;
use serde::Serialize;
use serde_json::json;
use std::{env, fmt::Display, fs, path::PathBuf};

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();

    let msg = handle_cli(cli).await?;
    println!("{msg}");

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

async fn handle_cli(cli: Cli) -> Result<Msg> {
    match cli.command {
        Commands::EnvInit => env_init().await,
        Commands::EnvInfo(args) => env_info(args),
        // File Creation
        Commands::GenerateKeypair(args) => generate_keypair(args),
        Commands::GenerateRoute(args) => generate_route(args),
        // API Commands
        Commands::GetRoutes(args) => get_routes(args).await,
        Commands::GetRoute(args) => get_route(args).await,
        Commands::GetOrgs(args) => get_orgs(args).await,
        Commands::GetOrg(args) => get_org(args).await,
        Commands::CreateRoute(args) => create_route(args).await,
        Commands::CreateHelium(args) => create_helium_org(args).await,
        Commands::CreateRoaming(args) => create_roaming_org(args).await,
        Commands::UpdateRoute(args) => update_route(args).await,
        Commands::RemoveRoute(args) => remove_route(args).await,
        // File updating commands
        Commands::Add { command } => match command {
            AddCommands::Devaddr(args) => add_devaddr(args).await,
            AddCommands::Eui(args) => add_eui(args).await,
            AddCommands::GwmpMapping(args) => add_gwmp_mapping(args).await,
            AddCommands::Protocol { command } => match command {
                ProtocolCommands::Http(args) => add_http_protocol(args).await,
                ProtocolCommands::Gwmp(args) => add_gwmp_protocol(args).await,
                ProtocolCommands::PacketRouter(args) => add_packet_router_protocol(args).await,
            },
        },
        // Helpers
        Commands::SubnetMask(args) => subnet_mask(args),
    }
}

fn subnet_mask(args: SubnetMask) -> Result<Msg> {
    if let (Some(start), Some(end)) = (args.start_addr, args.end_addr) {
        let devaddr_range = DevaddrRange::new(start, end)?;
        return Msg::ok(devaddr_range.to_subnet().pretty_json()?);
    }

    if let Some(path) = args.route_file {
        let routes = if path.is_file() {
            vec![Route::from_file(&path)?]
        } else {
            Route::from_dir(&path)?
        };

        let mut output = vec![];
        for route in routes {
            output.push(RouteSubnets::from_route(route))
        }
        return Msg::ok(output.pretty_json()?);
    }

    Msg::err("not enough arguments, run again with `--help`".to_string())
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
        .interact()?;
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

fn env_info(args: EnvInfo) -> Result<Msg> {
    let env_keypair = env::var(ENV_KEYPAIR_BIN).ok().map(|i| i.into());
    let (env_keypair_location, env_public_key) = get_keypair(env_keypair);
    let (arg_keypair_location, arg_public_key) = get_keypair(args.keypair);

    let output = json!({
        "environment": {
            ENV_CONFIG_HOST: env::var(ENV_CONFIG_HOST).unwrap_or_else(|_| "unset".into()),
            ENV_NET_ID:  env::var(ENV_NET_ID).unwrap_or_else(|_| "unset".into()),
            ENV_OUI:  env::var(ENV_OUI).unwrap_or_else(|_| "unset".into()),
            ENV_MAX_COPIES: env::var(ENV_MAX_COPIES).unwrap_or_else(|_| "unset".into()),
            ENV_KEYPAIR_BIN:  env_keypair_location,
            "public_key_from_keypair": env_public_key,
        },
        "arguments": {
            "config_host": args.config_host,
            "net_id": args.net_id,
            "oui": args.oui,
            "max_copies": args.max_copies,
            "keypair": arg_keypair_location,
            "public_key_from_keypair": arg_public_key
        }
    });
    Msg::ok(output.pretty_json()?)
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

async fn add_http_protocol(args: AddHttpSettings) -> Result<Msg> {
    let http = Protocol::make_http(args.flow_type, args.dedupe_timeout, args.path);
    let server = Server::new(args.host, args.port, http);

    if !args.commit {
        return Msg::ok(format!("valid http settings\n{}", server.pretty_json()?));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;

    Msg::ok(format!("{} written", args.route_file.display()))
}

async fn add_gwmp_protocol(args: AddGwmpSettings) -> Result<Msg> {
    let gwmp = match (args.region, args.region_port) {
        (Some(region), Some(region_port)) => Protocol::make_gwmp(region, region_port)?,
        (None, None) => Protocol::default_gwmp(),
        _ => return Msg::err("Must provide both `region` and `region_port`".to_string()),
    };
    let server = Server::new(args.host, args.port, gwmp);

    if !args.commit {
        return Msg::ok(format!("valid gwmp settings\n{}", server.pretty_json()?));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
    route.write(&args.route_file)?;

    Msg::ok(
        [
            format!("{} written", args.route_file.display()),
            "To add more region mapping, use the command `add gwmp-mapping`".to_string(),
        ]
        .join("\n"),
    )
}

async fn add_packet_router_protocol(args: AddPacketRouterSettings) -> Result<Msg> {
    let packet_router = Protocol::default_packet_router();
    let server = Server::new(args.host, args.port, packet_router);

    if !args.commit {
        return Msg::ok(format!(
            "valid packet router settings\n{}",
            server.pretty_json()?,
        ));
    }

    let mut route = Route::from_file(&args.route_file)?;
    route.set_server(server);
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

async fn get_routes(args: GetRoutes) -> Result<Msg> {
    let mut client = client::RouteClient::new(&args.config_host).await?;
    let route_list = client
        .list(args.oui, &args.owner, &args.keypair.to_keypair()?)
        .await?;

    if args.commit {
        route_list.write_all(&args.route_out_dir)?;
        return Msg::ok(format!("{} routes written", route_list.count()));
    }

    Msg::ok(route_list.pretty_json()?)
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

async fn get_orgs(args: GetOrgs) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host).await?;
    let org = client.list().await?;

    Msg::ok(org.pretty_json()?)
}

async fn get_org(args: GetOrg) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host).await?;
    let org = client.get(args.oui).await?;

    Msg::ok(org.pretty_json()?)
}

async fn create_route(args: CreateRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;

    if !route.id.is_empty() {
        return Msg::err("Route already has an ID, cannot be created".to_string());
    }

    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        match client
            .create_route(route, &args.owner, &args.keypair.to_keypair()?)
            .await
        {
            Ok(created_route) => {
                // Write to both locations to prevent re-creation of route after
                // ID is assigned.
                created_route.write(&args.route_out_dir)?;
                created_route.write(&args.route_file)?;

                return Msg::ok(format!(
                    "{}/{} written",
                    &args.route_out_dir.display(),
                    created_route.filename()
                ));
            }
            Err(err) => {
                // TODO: print this prettier
                return Msg::err(format!("route not created: {err}"));
            }
        }
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
            .push(route, &args.owner, &args.keypair.to_keypair()?)
            .await?;
        updated_route.write(args.route_file.as_path())?;
        return Msg::ok(format!("{} written", &args.route_file.display()));
    }
    Msg::ok(format!(
        "{} is valid, pass `--commit` to update",
        &args.route_file.display()
    ))
}

async fn remove_route(args: RemoveRoute) -> Result<Msg> {
    let route = Route::from_file(&args.route_file)?;
    if args.commit {
        let mut client = client::RouteClient::new(&args.config_host).await?;
        let removed_route = client
            .delete(&route.id, &args.owner, &args.keypair.to_keypair()?)
            .await?;
        removed_route.remove(
            args.route_file
                .parent()
                .expect("filename is in a directory"),
        )?;
        return Msg::ok(format!("{} deleted", &args.route_file.display()));
    }
    Msg::ok(format!(
        "{} ready for deletion, pass `--commit` to remove",
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
        let created_org = client
            .create_roamer(
                &args.owner,
                &args.payer,
                args.net_id.into(),
                args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(
            [
                "== Roaming Organization Created ==".to_string(),
                created_org.pretty_json()?,
                "== Environment Variables ==".to_string(),
                format!("{ENV_NET_ID}={}", created_org.net_id),
                format!("{ENV_OUI}={}", created_org.org.oui),
            ]
            .join("\n"),
        );
    }
    Msg::ok("pass `--commit` to create Roaming organization".to_string())
}

fn get_keypair(path: Option<PathBuf>) -> (String, String) {
    match path {
        None => ("unset".to_string(), "unset".to_string()),
        Some(path) => {
            let display_path = path.as_path().display().to_string();
            match fs::read(path).with_context(|| format!("path does not exist: {display_path}")) {
                Err(e) => (e.to_string(), "".to_string()),
                Ok(data) => match Keypair::try_from(&data[..]) {
                    Err(e) => (display_path, e.to_string()),
                    Ok(keypair) => (display_path, keypair.public_key().to_string()),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{env_info, generate_keypair, get_keypair, Msg};
    use helium_config_service_cli::{
        cmds::{self, EnvInfo, GenerateKeypair},
        hex_field,
    };
    use std::{env, fs};
    use temp_dir::TempDir;

    // Allow tests to get inside the container
    impl Msg {
        fn into_inner(self) -> String {
            match self {
                Msg::Success(s) => s,
                Msg::Error(s) => s,
            }
        }
    }

    #[test]
    fn env_info_test() {
        // Make the keypairs to be referenced
        let dir = TempDir::new().unwrap();
        let env_keypair = dir.child("env-keypair.bin");
        let arg_keypair = dir.child("arg-keypair.bin");
        generate_keypair(GenerateKeypair {
            out_file: env_keypair.clone(),
            commit: true,
        })
        .unwrap();
        generate_keypair(GenerateKeypair {
            out_file: arg_keypair.clone(),
            commit: true,
        })
        .unwrap();

        // Set the environment and arguments
        env::set_var(cmds::ENV_CONFIG_HOST, "env-localhost:1337");
        env::set_var(cmds::ENV_NET_ID, "C0053");
        env::set_var(cmds::ENV_OUI, "42");
        env::set_var(cmds::ENV_MAX_COPIES, "42");
        env::set_var(cmds::ENV_KEYPAIR_BIN, env_keypair.clone());

        let env_args = EnvInfo {
            config_host: Some("arg-localhost:1337".to_string()),
            keypair: Some(arg_keypair.clone()),
            net_id: Some(hex_field::net_id(42)),
            oui: Some(4),
            max_copies: Some(1337),
        };

        // =======
        let output = env_info(env_args).unwrap().into_inner();
        let s: serde_json::Value = serde_json::from_str(&output.to_string()).unwrap();

        let env = &s["environment"];
        let arg = &s["arguments"];

        let string_not_empty =
            |val: &serde_json::Value| !val.as_str().unwrap().to_string().is_empty();

        assert_eq!(env[cmds::ENV_CONFIG_HOST], "env-localhost:1337");
        assert_eq!(env[cmds::ENV_NET_ID], "C0053");
        assert_eq!(env[cmds::ENV_OUI], "42");
        assert_eq!(env[cmds::ENV_MAX_COPIES], "42");
        assert_eq!(
            env[cmds::ENV_KEYPAIR_BIN],
            env_keypair.display().to_string()
        );
        assert!(string_not_empty(&env["public_key_from_keypair"]));

        assert_eq!(arg["config_host"], "arg-localhost:1337");
        assert_eq!(arg["keypair"], arg_keypair.display().to_string());
        assert!(string_not_empty(&arg["public_key_from_keypair"]));
        assert_eq!(arg["net_id"], "00002A");
        assert_eq!(arg["oui"], 4);
        assert_eq!(arg["max_copies"], 1337);
    }

    #[test]
    fn get_keypair_does_not_exist() {
        let (location, pubkey) = get_keypair(Some("./nowhere.bin".into()));
        assert_eq!(location, "path does not exist: ./nowhere.bin");
        assert!(pubkey.is_empty());
    }

    #[test]
    fn get_keypair_invalid() {
        // Write an invalid keypair
        let dir = TempDir::new().unwrap();
        let arg_keypair = dir.child("arg-keypair.bin");
        fs::write(arg_keypair.clone(), "invalid key").unwrap();

        // =======
        let (location, pubkey) = get_keypair(Some(arg_keypair.clone()));
        assert_eq!(location, arg_keypair.display().to_string());
        assert_eq!(pubkey, "decode error");
    }

    #[test]
    fn get_keypair_not_provided() {
        let (location, pubkey) = get_keypair(None);
        assert_eq!(location, "unset");
        assert_eq!(pubkey, "unset");
    }
}
