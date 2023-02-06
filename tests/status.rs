use helium_config_service_cli::{
    cmds::{self, *},
    hex_field, Result,
};
use temp_dir::TempDir;
use tracing::info;

mod common;

#[tokio::test]
async fn create_route_and_update_server() -> Result {
    tracing_subscriber::fmt::init();

    let working_dir = TempDir::new()?;
    let keypair_path = working_dir.child("keypair.bin");
    let config_host = common::CONFIG_HOST.to_string();

    // Generate keypair
    let public_key = common::generate_keypair(keypair_path.clone())?;

    // Create an org and ensure we start out with no routes
    let org_res = common::create_helium_org(&public_key, 16, keypair_path.clone()).await?;
    common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route and ensure there's no default protocol
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;
    let out = cmds::route::get_route(GetRoute {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
    })
    .await?;
    info!("{out}");
    assert!(route.active);

    // Disable the Route
    let out = cmds::route::deactivate_route(DeactivateRoute {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out}");
    let route = common::get_route(&route.id, keypair_path.clone()).await?;
    assert!(!route.active);

    // Re-enable to the Route
    let out = cmds::route::activate_route(ActivateRoute {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out}");
    let route = common::get_route(&route.id, keypair_path.clone()).await?;
    assert!(route.active);

    Ok(())
}
