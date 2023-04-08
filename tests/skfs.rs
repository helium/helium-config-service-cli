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

    // List session key filters, there are none
    let out = cmds::session_key_filter::list_filters(ListFilters {
        oui: org_res.org.oui,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("empty list: {out}");
    let filters = skf_client
        .list_filters(org_res.org.oui, &keypair_path.to_keypair()?)
        .await?;
    assert!(filters.is_empty());

    // Add 2 session key filters
    let out = cmds::session_key_filter::add_filter(AddFilter {
        oui: org_res.org.oui,
        devaddr: hex_field::devaddr(1),
        session_key: "key-one".to_string(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("add 1: {out}");

    let out = cmds::session_key_filter::add_filter(AddFilter {
        oui: org_res.org.oui,
        devaddr: hex_field::devaddr(2),
        session_key: "key-two".to_string(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("add 2: {out}");

    // List session key filters again, expecting 2
    let out = cmds::session_key_filter::list_filters(ListFilters {
        oui: org_res.org.oui,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("list of 2: {out}");
    let filters = skf_client
        .list_filters(org_res.org.oui, &keypair_path.to_keypair()?)
        .await?;
    assert_eq!(2, filters.len());

    // Get specific devaddr, expecting 1
    let out = cmds::session_key_filter::get_filters(GetFilters {
        oui: org_res.org.oui,
        devaddr: hex_field::devaddr(1),
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("get, list of 1: {out}");
    let filters = skf_client
        .get_filters(
            org_res.org.oui,
            hex_field::devaddr(1),
            &keypair_path.to_keypair()?,
        )
        .await?;
    assert_eq!(1, filters.len());

    // Remove both session key filters
    let out = cmds::session_key_filter::remove_filter(RemoveFilter {
        oui: org_res.org.oui,
        devaddr: hex_field::devaddr(1),
        session_key: "key-one".to_string(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
        keypair: keypair_path.clone(),
        commit: true,
    })
    .await?;
    info!("removing 1: {out}");

    let out = cmds::session_key_filter::remove_filter(RemoveFilter {
        oui: org_res.org.oui,
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
    let out = cmds::session_key_filter::list_filters(ListFilters {
        oui: org_res.org.oui,
        keypair: keypair_path.clone(),
        config_host: config_host.clone(),
        config_pubkey: config_pubkey.clone(),
    })
    .await?;
    info!("empty list: {out}");
    let filters = skf_client
        .list_filters(org_res.org.oui, &keypair_path.to_keypair()?)
        .await?;
    assert!(filters.is_empty());

    Ok(())
}
