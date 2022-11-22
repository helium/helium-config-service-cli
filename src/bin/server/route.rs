use std::{pin::Pin, sync::Arc};

use helium_config_service_cli::{route::Route, Result};
use helium_proto::services::config::{
    route_server, RouteCreateReqV1, RouteDeleteReqV1, RouteGetReqV1, RouteListReqV1,
    RouteListResV1, RouteStreamReqV1, RouteStreamResV1, RouteUpdateReqV1, RouteV1,
};
use tonic::{codegen::futures_core::Stream, Response, Status};
use tracing::info;

use crate::storage::{RouteStorage, Storage};

#[derive(Debug, Default)]
pub struct RouteService {
    storage: Arc<Storage>,
}

impl RouteService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
}

#[tonic::async_trait]
impl route_server::Route for RouteService {
    async fn list(
        &self,
        request: tonic::Request<RouteListReqV1>,
    ) -> Result<tonic::Response<RouteListResV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting routes");
        match self.storage.get_routes(req.oui) {
            Ok(routes) => Ok(Response::new(RouteListResV1 {
                routes: routes.iter().map(|r| r.to_owned().into()).collect(),
            })),
            Err(e) => Err(Status::not_found(format!("no routes: {e}"))),
        }
    }
    async fn get(
        &self,
        request: tonic::Request<RouteGetReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        let req = request.into_inner();
        info!("getting route");
        match self.storage.get_route(req.id) {
            Some(route) => {
                info!("found");
                Ok(Response::new(route.into()))
            }
            _ => {
                info!("does not exist");
                Err(Status::not_found("no route"))
            }
        }
    }
    async fn create(
        &self,
        request: tonic::Request<RouteCreateReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "creating route");

        let route: Route = req.route.expect("route to create").into();
        match self.storage.create_route(req.oui, route) {
            Ok(route) => Ok(Response::new(route.into())),
            Err(e) => Err(Status::not_found(format!("could not find: {e}"))),
        }
    }
    async fn update(
        &self,
        request: tonic::Request<RouteUpdateReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        let req = request.into_inner();
        info!("updating route");

        let route: Route = req.route.expect("route to update").into();
        match self.storage.update_route(route) {
            Ok(route) => Ok(Response::new(route.into())),
            Err(e) => Err(Status::not_found(format!("could not find: {e}"))),
        }
    }
    async fn delete(
        &self,
        request: tonic::Request<RouteDeleteReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        let req = request.into_inner();
        info!("delete route");

        match self.storage.delete_route(req.id) {
            Some(route) => Ok(Response::new(route.into())),
            None => Err(Status::not_found("no route")),
        }
    }
    ///Server streaming response type for the stream method.
    type streamStream = Pin<Box<dyn Stream<Item = Result<RouteStreamResV1, Status>> + Send>>;
    async fn stream(
        &self,
        _request: tonic::Request<RouteStreamReqV1>,
    ) -> Result<tonic::Response<Self::streamStream>, tonic::Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Stream not implemented",
        ))
    }
}
