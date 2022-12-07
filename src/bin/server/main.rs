use helium_config_service_cli::Result;
use helium_proto::services::config::{
    org_server::OrgServer, route_server::RouteServer,
    session_key_filter_server::SessionKeyFilterServer,
};
use std::sync::Arc;
use storage::Storage;
use tonic::transport::Server;

mod org;
mod route;
mod skf;
mod storage;

#[tokio::main]
async fn main() -> Result {
    let (route_tx, _) = tokio::sync::broadcast::channel(128);
    let (filter_tx, _) = tokio::sync::broadcast::channel(128);
    let route_updates = Arc::new(route_tx);
    let filter_updates = Arc::new(filter_tx);
    let store = Arc::new(Storage::new(route_updates.clone(), filter_updates.clone()));

    tracing_subscriber::fmt::init();

    let address = "0.0.0.0:50051".parse()?;
    let org_service = org::OrgService::new(store.clone());
    let route_service = route::RouteService::new(store.clone());
    let skf_service = skf::SKFService::new(store.clone());

    Server::builder()
        .add_service(OrgServer::new(org_service))
        .add_service(RouteServer::new(route_service))
        .add_service(SessionKeyFilterServer::new(skf_service))
        .serve(address)
        .await?;
    Ok(())
}
