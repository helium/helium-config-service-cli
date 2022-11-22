use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use helium_config_service_cli::Result;
use helium_proto::services::config::{org_server::OrgServer, route_server::RouteServer};
#[allow(unused_imports)]
use tonic::{codegen::futures_core::Stream, transport::Server, Request, Response, Status};

mod org;
mod route;

#[tokio::main]
async fn main() -> Result {
    let store = Arc::new(RwLock::new(HashMap::new()));

    tracing_subscriber::fmt::init();

    let address = "0.0.0.0:50051".parse()?;
    let org_service = org::OrgService::new(store.clone());
    let route_service = route::RouteService::default();

    Server::builder()
        .add_service(OrgServer::new(org_service))
        .add_service(RouteServer::new(route_service))
        .serve(address)
        .await?;
    Ok(())
}
