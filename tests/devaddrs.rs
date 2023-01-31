use helium_config_service_cli::{
    client,
    cmds::{self, *},
    hex_field, DevaddrRange, Result,
};

use temp_dir::TempDir;
use tracing::info;

mod common;

#[tokio::test]
async fn create_route_and_add_remove_devaddrs() -> Result {
    // tracing_subscriber::fmt::init();

    let working_dir = TempDir::new()?;
    let keypair_path = working_dir.child("keypair.bin");
    let config_host = common::CONFIG_HOST.to_string();

    let mut devaddr_client = client::DevaddrClient::new(&config_host).await?;

    // Generate keypair
    let public_key = common::generate_keypair(keypair_path.clone())?;

    // Create an org and ensure we start out with no routes
    let org_res = common::create_helium_org(&public_key, 16, keypair_path.clone()).await?;
    let constraint = org_res.devaddr_constraints.first().unwrap();
    common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route and ensure there's no default devaddrs
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;
    common::ensure_no_devaddrs(&route.id, keypair_path.clone()).await?;

    // devaddr outside org constraint, should not add
    let out1 = cmds::route::devaddrs::add_devaddr(AddDevaddr {
        start_addr: hex_field::devaddr(1),
        end_addr: hex_field::devaddr(2),
        route_id: route.id.clone(),
        config_host: config_host.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    println!("1: {out1}");
    common::ensure_no_devaddrs(&route.id, keypair_path.clone()).await?;

    // Construct a devaddr within the org contraint, add and remove
    let devaddr_range = constraint.start_addr.to_range(3);
    let out2 = cmds::route::devaddrs::add_devaddr(AddDevaddr {
        start_addr: devaddr_range.start_addr,
        end_addr: devaddr_range.end_addr,
        route_id: route.id.clone(),
        config_host: config_host.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    println!("2: {out2}");
    common::ensure_num_devaddrs(1, &route.id, keypair_path.clone()).await?;

    let out3 = cmds::route::devaddrs::remove_devaddr(RemoveDevaddr {
        start_addr: devaddr_range.start_addr,
        end_addr: devaddr_range.end_addr,
        route_id: route.id.clone(),
        config_host: config_host.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    println!("3: {out3}");
    common::ensure_no_devaddrs(&route.id, keypair_path.clone()).await?;

    // Add many devaddrs to delete
    let mut devaddrs = vec![];
    for d in 1..10 {
        let range = constraint.start_addr.to_range(d);
        let range = DevaddrRange::new(route.id.clone(), range.start_addr, range.end_addr)?;

        devaddrs.push(range);
    }
    let adding = devaddr_client
        .add_devaddrs(devaddrs, &keypair_path.to_keypair()?)
        .await?;
    info!("bulk adding devaddrs: {adding:?}");
    let _ = common::ensure_num_devaddrs(9, &route.id, keypair_path.clone()).await;

    // Print subnets for visual inspection
    let out4 = cmds::route::devaddrs::subnet_mask(RouteSubnetMask {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
    })
    .await?;
    info!("4: {out4}");

    let out5 = cmds::route::devaddrs::clear_devaddrs(ClearDevaddrs {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("5: {out5}");
    let _ = common::ensure_no_devaddrs(&route.id, keypair_path.clone()).await;

    Ok(())
}
