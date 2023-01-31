#![allow(unused)]
use std::{path::PathBuf, str::FromStr};
use helium_config_service_cli::{
    client,
    cmds::{self, *},
    hex_field,
    route::Route,
    OrgResponse, Result,
};
use helium_crypto::PublicKey;
use tracing::info;

pub const CONFIG_HOST: &str = "http://127.0.0.1:50051";

/// These helpers use the CLI commands _and_ client methods directly.
///
/// You can run these against a fresh test server with
/// `cargo run --bin server`
///
/// The CLI command output can be printed by enabling tracing in the test.
///
/// ```
/// tracing_subscriber::fmt::init();
/// ```
///
/// The clients are used so the test can use information from the config service directly.

pub fn generate_keypair(path: PathBuf) -> Result<PublicKey> {
    let out = cmds::env::generate_keypair(cmds::GenerateKeypair {
        out_file: path.clone(),
        commit: true,
    })?;
    info!("generate_keypair: {out}");
    let (_, public_key) = cmds::env::get_public_key_from_path(Some(path));
    let public_key = PublicKey::from_str(&public_key)?;
    Ok(public_key)
}

pub async fn create_helium_org(
    public_key: &PublicKey,
    devaddr_count: u64,
    keypair_path: PathBuf,
) -> Result<OrgResponse> {
    let out = cmds::org::create_helium_org(CreateHelium {
        owner: public_key.clone(),
        payer: public_key.clone(),
        devaddr_count,
        keypair: keypair_path,
        config_host: CONFIG_HOST.to_string(),
        commit: true,
    })
    .await?;
    info!("{out}");

    let mut org_client = client::OrgClient::new(CONFIG_HOST).await?;
    let org_list = org_client.list().await?;
    let org = org_list.orgs.last().expect("existing org after creation");
    // we want the devaddr constraints
    let res = org_client.get(org.oui).await?;
    Ok(res)
}

pub async fn ensure_no_routes(oui: u64, keypair_path: PathBuf) -> Result {
    let out = cmds::route::list_routes(ListRoutes {
        oui,
        keypair: keypair_path.clone(),
        config_host: CONFIG_HOST.to_string(),
        commit: false,
    })
    .await?;
    info!("{out}");

    let mut route_client = client::RouteClient::new(CONFIG_HOST).await?;
    let route_list = route_client.list(oui, &keypair_path.to_keypair()?).await?;
    assert!(route_list.routes.is_empty());
    Ok(())
}

pub async fn create_empty_route(
    net_id: hex_field::HexNetID,
    oui: u64,
    keypair_path: PathBuf,
) -> Result<Route> {
    let out1 = cmds::route::new_route(NewRoute {
        net_id,
        oui,
        max_copies: 5,
        keypair: keypair_path.clone(),
        config_host: CONFIG_HOST.to_string(),
        commit: true,
    })
    .await?;
    info!("{out1}");

    let mut route_client = client::RouteClient::new(CONFIG_HOST).await?;
    let route_list = route_client.list(oui, &keypair_path.to_keypair()?).await?;
    Ok(route_list
        .routes
        .first()
        .expect("route created through CLI commands")
        .to_owned())
}

pub async fn get_route(route_id: &str, keypair_path: PathBuf) -> Result<Route> {
    let mut route_client = client::RouteClient::new(CONFIG_HOST).await?;
    let route = route_client
        .get(route_id, &keypair_path.to_keypair()?)
        .await?;
    Ok(route)
}

pub async fn ensure_no_euis(route_id: &str, keypair_path: PathBuf) -> Result {
    ensure_num_euis(0, route_id, keypair_path).await
}

pub async fn ensure_no_devaddrs(route_id: &str, keypair_path: PathBuf) -> Result {
    ensure_num_devaddrs(0, route_id, keypair_path).await
}

pub async fn ensure_num_euis(eui_count: usize, route_id: &str, keypair_path: PathBuf) -> Result {
    let out = cmds::route::euis::list_euis(ListEuis {
        route_id: route_id.to_string(),
        keypair: keypair_path.clone(),
        config_host: CONFIG_HOST.to_string(),
    })
    .await?;
    info!("{out}");

    let mut eui_client = client::EuiClient::new(CONFIG_HOST).await?;
    let euis = eui_client
        .get_euis(route_id, &keypair_path.to_keypair()?)
        .await?;
    assert_eq!(eui_count, euis.len());
    Ok(())
}

pub async fn ensure_num_devaddrs(
    devaddr_count: usize,
    route_id: &str,
    keypair_path: PathBuf,
) -> Result {
    let out = cmds::route::devaddrs::list_devaddrs(ListDevaddrs {
        route_id: route_id.to_string(),
        keypair: keypair_path.clone(),
        config_host: CONFIG_HOST.to_string(),
    })
    .await?;
    info!("{out}");

    let mut devaddr_client = client::DevaddrClient::new(CONFIG_HOST).await?;
    let addrs = devaddr_client
        .get_devaddrs(route_id, &keypair_path.to_keypair()?)
        .await?;
    assert_eq!(devaddr_count, addrs.len());
    Ok(())
}
