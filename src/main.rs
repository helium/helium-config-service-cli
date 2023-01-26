use clap::Parser;
use helium_config_service_cli::{
    cmds::{
        self, env, org,
        route::{self, devaddrs, euis},
        Cli, Commands, EnvCommands as Env, OrgCommands as Org, RouteCommands, RouteUpdateCommand,
    },
    Msg, Result,
};

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();

    let msg = handle_cli(cli).await?;
    println!("{msg}");

    Ok(())
}

pub async fn handle_cli(cli: Cli) -> Result<Msg> {
    match cli.command {
        Commands::Env { command } => match command {
            Env::Init => env::env_init().await,
            Env::Info(args) => env::env_info(args),
            Env::GenerateKeypair(args) => env::generate_keypair(args),
        },
        Commands::Route { command } => match command {
            RouteCommands::List(args) => route::list_routes(args).await,
            RouteCommands::Get(args) => route::get_route(args).await,
            RouteCommands::New(args) => route::new_route(args).await,
            RouteCommands::Delete(args) => route::delete_route(args).await,
            RouteCommands::Update { command } => match command {
                RouteUpdateCommand::MaxCopies(args) => route::update_max_copies(args).await,
                RouteUpdateCommand::Server(args) => route::update_server(args).await,
                RouteUpdateCommand::Http(args) => route::update_http(args).await,
                RouteUpdateCommand::AddGwmpRegion(args) => route::add_gwmp_region(args).await,
                RouteUpdateCommand::RemoveGwmpRegion(args) => route::remove_gwmp_region(args).await,
                RouteUpdateCommand::PacketRouter(args) => route::update_packet_router(args).await,
            },
            RouteCommands::Euis { command } => match command {
                cmds::EuiCommands::List(args) => euis::list_euis(args).await,
                cmds::EuiCommands::Add(args) => euis::add_eui(args).await,
                cmds::EuiCommands::Remove(args) => euis::delete_eui(args).await,
                cmds::EuiCommands::Clear(args) => euis::clear_euis(args).await,
            },
            RouteCommands::Devaddrs { command } => match command {
                cmds::DevaddrCommands::List(args) => devaddrs::list_devaddrs(args).await,
                cmds::DevaddrCommands::Add(args) => devaddrs::add_devaddr(args).await,
                cmds::DevaddrCommands::Delete(args) => devaddrs::delete_devaddr(args).await,
                cmds::DevaddrCommands::SubnetMask(args) => devaddrs::subnet_mask(args).await,
                cmds::DevaddrCommands::Clear(args) => devaddrs::clear_devaddrs(args).await,
            },
        },
        Commands::Org { command } => match command {
            Org::List(args) => org::list_orgs(args).await,
            Org::Get(args) => org::get_org(args).await,
            Org::CreateHelium(args) => org::create_helium_org(args).await,
            Org::CreateRoaming(args) => org::create_roaming_org(args).await,
        },
        Commands::SubnetMask(args) => cmds::subnet_mask(args),
    }
}
