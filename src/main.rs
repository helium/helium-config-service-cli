use clap::Parser;
use helium_config_service_cli::{
    cmds::{
        self, admin, env, gateway, org,
        route::{self, devaddrs, euis, skfs},
        Cli, Commands, EnvCommands as Env, OrgCommands as Org, RouteCommands, RouteUpdateCommand,
    },
    Msg, Result,
};

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();

    if cli.print_command {
        println!("{cli:#?}");
    }

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
                RouteUpdateCommand::IgnoreEmptySkf(args) => {
                    route::update_ignore_empty_skf(args).await
                }
            },
            RouteCommands::Euis { command } => match command {
                cmds::EuiCommands::List(args) => euis::list_euis(args).await,
                cmds::EuiCommands::Add(args) => euis::add_eui(args).await,
                cmds::EuiCommands::Remove(args) => euis::remove_eui(args).await,
                cmds::EuiCommands::Clear(args) => euis::clear_euis(args).await,
            },
            RouteCommands::Devaddrs { command } => match command {
                cmds::DevaddrCommands::List(args) => devaddrs::list_devaddrs(args).await,
                cmds::DevaddrCommands::Add(args) => devaddrs::add_devaddr(args).await,
                cmds::DevaddrCommands::Remove(args) => devaddrs::remove_devaddr(args).await,
                cmds::DevaddrCommands::SubnetMask(args) => devaddrs::subnet_mask(args).await,
                cmds::DevaddrCommands::Clear(args) => devaddrs::clear_devaddrs(args).await,
            },
            RouteCommands::Activate(args) => route::activate_route(args).await,
            RouteCommands::Deactivate(args) => route::deactivate_route(args).await,
            RouteCommands::Skfs { command } => match command {
                cmds::SkfCommands::List(args) => skfs::list_filters(args).await,
                cmds::SkfCommands::Get(args) => skfs::get_filters(args).await,
                cmds::SkfCommands::Add(args) => skfs::add_filter(args).await,
                cmds::SkfCommands::Remove(args) => skfs::remove_filter(args).await,
                cmds::SkfCommands::Clear(args) => skfs::clear_filters(args).await,
                cmds::SkfCommands::Update(args) => skfs::update_filters_from_file(args).await,
            },
        },
        Commands::Org { command } => match command {
            Org::List(args) => org::list_orgs(args).await,
            Org::Get(args) => org::get_org(args).await,
            Org::CreateHelium(args) => org::create_helium_org(args).await,
            Org::CreateRoaming(args) => org::create_roaming_org(args).await,
            Org::Enable(args) => org::enable_org(args).await,
            Org::Update { command } => match command {
                cmds::OrgUpdateCommand::Owner(args) => org::update_owner(args).await,
                cmds::OrgUpdateCommand::Payer(args) => org::update_payer(args).await,
                cmds::OrgUpdateCommand::DelegateAdd(args) => org::add_delegate_key(args).await,
                cmds::OrgUpdateCommand::DelegateRemove(args) => {
                    org::remove_delegate_key(args).await
                }
                cmds::OrgUpdateCommand::DevaddrSlabAdd(args) => org::add_devaddr_slab(args).await,
                cmds::OrgUpdateCommand::DevaddrConstraintAdd(args) => {
                    org::add_devaddr_constraint(args).await
                }
                cmds::OrgUpdateCommand::DevaddrConstraintRemove(args) => {
                    org::remove_devaddr_constraint(args).await
                }
            },
        },
        Commands::SubnetMask(args) => cmds::subnet_mask(args),
        Commands::Admin { command } => match command {
            cmds::AdminCommands::LoadRegion(args) => admin::load_region(args).await,
            cmds::AdminCommands::AddKey(args) => admin::add_key(args).await,
            cmds::AdminCommands::RemoveKey(args) => admin::remove_key(args).await,
        },
        Commands::Gateway { command } => match command {
            cmds::GatewayCommands::Location(args) => gateway::location(args).await,
            cmds::GatewayCommands::Info(args) => gateway::info(args).await,
        },
    }
}
