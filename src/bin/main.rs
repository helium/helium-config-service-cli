use clap::Parser;
use helium_config_service_cli::{
    cmds::{
        self, AddCommands as Add, Cli, Commands, EnvCommands as Env, OrgCommands as Org,
        ProtocolCommands as Protocol, RouteCommands as Route,
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

async fn handle_cli(cli: Cli) -> Result<Msg> {
    match cli.command {
        Commands::Env { command } => match command {
            Env::Init => cmds::env::env_init().await,
            Env::Info(args) => cmds::env::env_info(args),
            Env::GenerateKeypair(args) => cmds::env::generate_keypair(args),
        },
        Commands::Route { command } => match command {
            Route::Generate(args) => cmds::route::generate_route(args),
            Route::List(args) => cmds::route::get_routes(args).await,
            Route::Get(args) => cmds::route::get_route(args).await,
            Route::Create(args) => cmds::route::create_route(args).await,
            Route::Update(args) => cmds::route::update_route(args).await,
            Route::Remove(args) => cmds::route::remove_route(args).await,
            Route::SubnetMask(args) => cmds::route::subnet_mask(args),
            Route::Add { command } => match command {
                Add::Protocol { command } => match command {
                    Protocol::Http(args) => cmds::protocol::add_http_protocol(args).await,
                    Protocol::Gwmp(args) => cmds::protocol::add_gwmp_protocol(args).await,
                    Protocol::PacketRouter(args) => {
                        cmds::protocol::add_packet_router_protocol(args).await
                    }
                },
                Add::Devaddr(args) => cmds::route::add_devaddr(args).await,
                Add::Eui(args) => cmds::route::add_eui(args).await,
                Add::GwmpMapping(args) => cmds::route::add_gwmp_mapping(args).await,
            },
        },
        Commands::Org { command } => match command {
            Org::List(args) => cmds::org::get_orgs(args).await,
            Org::Get(args) => cmds::org::get_org(args).await,
            Org::CreateHelium(args) => cmds::org::create_helium_org(args).await,
            Org::CreateRoaming(args) => cmds::org::create_roaming_org(args).await,
        },
    }
}
