use helium_config_service_cli::Result;
use helium_proto::services::config::{org_server::OrgServer, route_server::RouteServer};
use std::sync::Arc;
use storage::Storage;
use tonic::transport::Server;

mod org;
mod route;
mod storage;

#[tokio::main]
async fn main() -> Result {
    let (tx, _) = tokio::sync::broadcast::channel(128);
    let route_updates = Arc::new(tx);
    let store = Arc::new(Storage::new(route_updates.clone()));

    tracing_subscriber::fmt::init();

    let address = "0.0.0.0:50051".parse()?;
    let org_service = org::OrgService::new(store.clone());
    let route_service = route::RouteService::new(store.clone());

    Server::builder()
        .add_service(OrgServer::new(org_service))
        .add_service(RouteServer::new(route_service))
        .serve(address)
        .await?;
    Ok(())
}
