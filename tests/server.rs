use helium_config_service_cli::{
    cmds::{self, *},
    hex_field, Result,
};
use temp_dir::TempDir;
use tracing::info;

mod common;

#[tokio::test]
async fn create_route_and_update_server() -> Result {
    // tracing_subscriber::fmt::init();

    let working_dir = TempDir::new()?;
    let keypair_path = working_dir.child("keypair.bin");
    let config_host = common::CONFIG_HOST.to_string();

    // Generate keypair
    let public_key = common::generate_keypair(keypair_path.clone())?;

    // Create an org and ensure we start out with no routes
    let org_res = common::create_helium_org(&public_key, 16, keypair_path.clone()).await?;
    let _ = common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route and ensure there's no default server
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;
    let out1 = cmds::route::get_route(GetRoute {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
    })
    .await?;
    info!("{out1}");
    assert!(route.server.host.is_empty());
    assert!(route.server.port == 0);

    // Update the server and port
    let out2 = cmds::route::update_server(UpdateServer {
        route_id: route.id.clone(),
        host: "www.example.com".to_string(),
        port: 1337,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out2}");
    let updated_route = common::get_route(&route.id, keypair_path.clone()).await?;
    assert_eq!("www.example.com", updated_route.server.host);
    assert_eq!(1337, updated_route.server.port);

    Ok(())
}
