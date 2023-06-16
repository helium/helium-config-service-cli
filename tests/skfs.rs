use helium_config_service_cli::{
    client,
    cmds::{self, *},
    hex_field, Result,
};

use temp_dir::TempDir;
use tracing::info;

mod common;

#[tokio::test]
async fn create_org_and_add_remove_session_key_filtesr() -> Result {
    // tracing_subscriber::fmt::init();

    let working_dir = TempDir::new()?;
    let keypair_path = working_dir.child("keypair.bin");
    let config_host = common::CONFIG_HOST.to_string();
    let config_pubkey = common::CONFIG_PUBKEY.to_string();

    let mut skf_client = client::SkfClient::new(&config_host, &config_pubkey).await?;

    // Generate keypair
    let public_key = common::generate_keypair(keypair_path.clone())?;

    // Create an org and ensure we start out with no routes
    let org_res = common::create_helium_org(&public_key, 16, keypair_path.clone()).await?;
    common::ensure_no_routes(org_res.org.oui, keypair_path.clone()).await?;

    // Create a route and ensure there's no default skfs
    let net_id = hex_field::net_id(0xC00053);
    let route = common::create_empty_route(net_id, org_res.org.oui, keypair_path.clone()).await?;

    // List session key filters, there are none
    let out = cmds::route::skfs::list_filters(ListFilters {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("empty list: {out}");
    let filters = skf_client
        .list_filters(&route.id, &keypair_path.to_keypair()?)
        .await?;
    assert!(filters.is_empty());

    // Add 2 session key filters
    let out = cmds::route::skfs::add_filter(AddFilter {
        route_id: route.id.clone(),
        devaddr: hex_field::devaddr(1),
        session_key: "key-one".to_string(),
        max_copies: Some(3),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("add 1: {out}");

    let out = cmds::route::skfs::add_filter(AddFilter {
        route_id: route.id.clone(),
        devaddr: hex_field::devaddr(2),
        session_key: "key-two".to_string(),
        max_copies: Some(3),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("add 2: {out}");

    // List session key filters again, expecting 2
    let out = cmds::route::skfs::list_filters(ListFilters {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("list of 2: {out}");
    let filters = skf_client
        .list_filters(&route.id, &keypair_path.to_keypair()?)
        .await?;
    assert_eq!(2, filters.len());

    // Get specific devaddr, expecting 1
    let out = cmds::route::skfs::get_filters(GetFilters {
        route_id: route.id.clone(),
        devaddr: hex_field::devaddr(1),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("get, list of 1: {out}");
    let filters = skf_client
        .get_filters(
            &route.id,
            hex_field::devaddr(1),
            &keypair_path.to_keypair()?,
        )
        .await?;
    assert_eq!(1, filters.len());

    // Remove both session key filters
    let out = cmds::route::skfs::remove_filter(RemoveFilter {
        route_id: route.id.clone(),
        devaddr: hex_field::devaddr(1),
        session_key: "key-one".to_string(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("removing 1: {out}");

    let out = cmds::route::skfs::remove_filter(RemoveFilter {
        route_id: route.id.clone(),
        devaddr: hex_field::devaddr(2),
        session_key: "key-two".to_string(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("removing 2: {out}");

    // List session key filters, expecting none
    let out = cmds::route::skfs::list_filters(ListFilters {
        route_id: route.id.clone(),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("empty list: {out}");
    let filters = skf_client
        .list_filters(&route.id, &keypair_path.to_keypair()?)
        .await?;
    assert!(filters.is_empty());

    Ok(())
}
