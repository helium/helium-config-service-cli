use std::pin::Pin;

use helium_proto::services::config::{
    route_server::Route, RouteCreateReqV1, RouteDeleteReqV1, RouteGetReqV1, RouteListReqV1,
    RouteListResV1, RouteStreamReqV1, RouteStreamResV1, RouteUpdateReqV1, RouteV1,
};
use tonic::{codegen::futures_core::Stream, Response, Status};

#[derive(Debug, Default)]
pub struct RouteService {}

#[tonic::async_trait]
impl Route for RouteService {
    async fn list(
        &self,
        request: tonic::Request<RouteListReqV1>,
    ) -> Result<tonic::Response<RouteListResV1>, tonic::Status> {
        let r = request.into_inner();
        let mut routes = vec![];
        let ids = [
            "1234", "2345", "3456", "4567", "5678", "6789", "7890", "8901", "9012", "0123",
        ];
        for index in 0..10 {
            routes.push(RouteV1 {
                id: ids[index].into(),
                net_id: 1,
                devaddr_ranges: vec![],
                euis: vec![],
                oui: r.oui as u64,
                server: None,
                max_copies: index as u32,
                nonce: 1,
            });
        }
        Ok(Response::new(RouteListResV1 { routes }))
    }
    async fn get(
        &self,
        request: tonic::Request<RouteGetReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        let r = request.into_inner();
        Ok(Response::new(RouteV1 {
            id: r.id,
            net_id: 1,
            devaddr_ranges: vec![],
            euis: vec![],
            oui: 66,
            server: None,
            max_copies: 99 as u32,
            nonce: 1,
        }))
    }
    async fn create(
        &self,
        _request: tonic::Request<RouteCreateReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Create not implemented",
        ))
    }
    async fn update(
        &self,
        _request: tonic::Request<RouteUpdateReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Update not implemented",
        ))
    }
    async fn delete(
        &self,
        _request: tonic::Request<RouteDeleteReqV1>,
    ) -> Result<tonic::Response<RouteV1>, tonic::Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Delete not implemented",
        ))
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
