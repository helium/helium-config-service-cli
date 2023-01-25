use helium_config_service_cli::{
    cmds::{self, *},
    hex_field, server, Result,
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
    let _ = common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route and ensure there's no default protocol
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;
    let out1 = cmds::route::get_route(GetRoute {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
    })
    .await?;
    info!("{out1}");
    assert!(route.server.protocol.is_none());

    // Set packet-router protocol
    let out2 = cmds::route::update_packet_router(UpdatePacketRouter {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out2}");
    let packet_router_route = common::get_route(&route.id, keypair_path.clone()).await?;
    assert!(packet_router_route.server.protocol.is_some());

    // Set Http Protocol
    let out3 = cmds::route::update_http(UpdateHttp {
        route_id: route.id.clone(),
        dedupe_timeout: 234,
        path: "path".to_string(),
        auth_header: Some("test-header".to_string()),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out3}");
    let http_route = common::get_route(&route.id, keypair_path.clone()).await?;
    let http_protocol = http_route
        .server
        .protocol
        .expect("existing protocol")
        .into_http_inner()?;

    assert_eq!(
        server::Http {
            flow_type: server::FlowType::Async,
            dedupe_timeout: 234,
            path: "path".to_string(),
            auth_header: "test-header".to_string()
        },
        http_protocol
    );

    // Set GWMP protocol
    let out4 = cmds::route::add_gwmp_region(AddGwmpRegion {
        route_id: route.id.clone(),
        region: helium_config_service_cli::region::Region::As9231a,
        region_port: 9001,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out4}");
    let gwmp_route = common::get_route(&route.id, keypair_path.clone()).await?;
    let gwmp_protocol = gwmp_route
        .server
        .protocol
        .expect("existing protocol")
        .into_gwmp_inner()?;
    assert_eq!(1, gwmp_protocol.mapping.len());

    let out5 = cmds::route::add_gwmp_region(AddGwmpRegion {
        route_id: route.id.clone(),
        region: helium_config_service_cli::region::Region::Eu433,
        region_port: 9002,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out5}");
    let gwmp_route = common::get_route(&route.id, keypair_path.clone()).await?;
    let gwmp_protocol = gwmp_route
        .server
        .protocol
        .expect("existing protocol")
        .into_gwmp_inner()?;
    assert_eq!(2, gwmp_protocol.mapping.len());

    let out6 = cmds::route::remove_gwmp_region(RemoveGwmpRegion {
        route_id: route.id.clone(),
        region: helium_config_service_cli::region::Region::As9231a,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        commit: true,
    })
    .await?;
    info!("{out6}");
    let gwmp_route = common::get_route(&route.id, keypair_path.clone()).await?;
    let gwmp_protocol = gwmp_route
        .server
        .protocol
        .expect("existing protocol")
        .into_gwmp_inner()?;
    assert_eq!(1, gwmp_protocol.mapping.len());

    Ok(())
}
