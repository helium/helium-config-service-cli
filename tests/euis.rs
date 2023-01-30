use helium_config_service_cli::{
    client,
    cmds::{self, *},
    hex_field, Eui, Result,
};
use temp_dir::TempDir;
use tracing::info;

mod common;

#[tokio::test]
async fn create_route_and_add_remove_euis() -> Result {
    // tracing_subscriber::fmt::init();

    let working_dir = TempDir::new()?;
    let keypair_path = working_dir.child("keypair.bin");
    let config_host = common::CONFIG_HOST.to_string();

    // Generate keypair
    let public_key = common::generate_keypair(keypair_path.clone())?;

    // Create an org and ensure we start with no routes
    let org_res = common::create_helium_org(&public_key, 8, keypair_path.clone()).await?;
    common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route an ensure there's no default euis
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;
    common::ensure_no_euis(&route.id, keypair_path.clone()).await?;

    // Add an EUI
    let out1 = cmds::route::euis::add_eui(AddEui {
        dev_eui: hex_field::eui(1),
        app_eui: hex_field::eui(2),
        route_id: route.id.clone(),
        config_host: config_host.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("1: {out1}");
    common::ensure_num_euis(1, &route.id, keypair_path.clone()).await?;

    // Remove Eui
    let out2 = cmds::route::euis::remove_eui(RemoveEui {
        dev_eui: hex_field::eui(1),
        app_eui: hex_field::eui(2),
        route_id: route.id.clone(),
        config_host: config_host.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    println!("2: {out2}");
    common::ensure_no_euis(&route.id, keypair_path.clone()).await?;

    // Add many Euis to delete
    let mut eui_client = client::EuiClient::new(common::CONFIG_HOST).await?;
    let mut euis = vec![];
    for e in 0..15 {
        euis.push(Eui::new(
            route.id.clone(),
            hex_field::eui(e),
            hex_field::eui(e + 1),
        )?);
    }
    let adding = eui_client
        .add_euis(euis, &keypair_path.to_keypair()?)
        .await?;
    info!("bulk adding euis: {adding:?}");
    common::ensure_num_euis(15, &route.id, keypair_path.clone()).await?;

    eui_client
        .delete_euis(route.id.clone(), &keypair_path.to_keypair()?)
        .await?;
    common::ensure_no_euis(&route.id, keypair_path.clone()).await?;

    Ok(())
}
