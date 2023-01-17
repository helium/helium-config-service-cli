use helium_config_service_cli::{
    proto::{DevaddrRangeV1, EuiPairV1},
    route::Route,
    Result,
};
use helium_proto::services::iot_config::{
    route_server, ActionV1, RouteCreateReqV1, RouteDeleteDevaddrRangesReqV1, RouteDeleteEuisReqV1,
    RouteDeleteReqV1, RouteDevaddrRangesResV1, RouteEuisResV1, RouteGetDevaddrRangesReqV1,
    RouteGetEuisReqV1, RouteGetReqV1, RouteListReqV1, RouteListResV1, RouteStreamReqV1,
    RouteStreamResV1, RouteUpdateDevaddrRangesReqV1, RouteUpdateEuisReqV1, RouteUpdateReqV1,
    RouteV1,
};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::info;

use crate::storage::{RouteStorage, Storage};

#[derive(Debug)]
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
            Ok(routes) => {
                info!("routes: {routes:?}");
                Ok(Response::new(RouteListResV1 {
                    routes: routes.iter().map(|r| r.to_owned().into()).collect(),
                }))
            }
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
    type streamStream = ReceiverStream<Result<RouteStreamResV1, Status>>;
    async fn stream(
        &self,
        _request: tonic::Request<RouteStreamReqV1>,
    ) -> Result<tonic::Response<Self::streamStream>, tonic::Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut updates = self.storage.subscribe_to_routes();

        info!("Connected");

        tokio::spawn(async move {
            while let Ok(update) = updates.recv().await {
                info!("route updated");
                if (tx.send(Ok(update)).await).is_err() {
                    break;
                }
            }
            info!("Disconnected");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type get_euisStream = ReceiverStream<Result<EuiPairV1, Status>>;
    async fn get_euis(
        &self,
        request: tonic::Request<RouteGetEuisReqV1>,
    ) -> Result<tonic::Response<Self::get_euisStream>, tonic::Status> {
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let euis = self.storage.get_euis_for_route(&req.route_id);
        info!(route_id = req.route_id, "getting euis");

        tokio::spawn(async move {
            for eui in euis {
                tx.send(Ok(eui)).await.expect("eui sent")
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn update_euis(
        &self,
        request: tonic::Request<tonic::Streaming<RouteUpdateEuisReqV1>>,
    ) -> Result<tonic::Response<RouteEuisResV1>, tonic::Status> {
        let mut stream = request.into_inner();

        while let Ok(Some(update)) = stream.message().await {
            match update.action() {
                ActionV1::Add => {
                    let eui = update.euis.expect("eui to update exists");
                    let added = self.storage.add_eui(eui.clone());
                    info!(added, ?eui, "adding eui");
                }
                ActionV1::Remove => {
                    let eui = update.euis.expect("eui to update exists");
                    let removed = self.storage.remove_eui(eui.clone());
                    info!(removed, ?eui, "removing eui");
                }
            };
        }

        Ok(Response::new(RouteEuisResV1 {}))
    }

    async fn delete_euis(
        &self,
        request: tonic::Request<RouteDeleteEuisReqV1>,
    ) -> Result<tonic::Response<RouteEuisResV1>, tonic::Status> {
        let req = request.into_inner();

        self.storage.clear_euis_for_route(&req.route_id);
        info!(route_id = req.route_id, "removing all euis");

        Ok(Response::new(RouteEuisResV1 {}))
    }

    type get_devaddr_rangesStream = ReceiverStream<Result<DevaddrRangeV1, Status>>;
    async fn get_devaddr_ranges(
        &self,
        request: tonic::Request<RouteGetDevaddrRangesReqV1>,
    ) -> Result<tonic::Response<Self::get_devaddr_rangesStream>, tonic::Status> {
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let devaddrs = self.storage.get_devaddrs_for_route(&req.route_id);
        info!(route_id = req.route_id, "getting devaddrs");

        tokio::spawn(async move {
            for devaddr in devaddrs {
                tx.send(Ok(devaddr)).await.expect("devaddr sent")
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn update_devaddr_ranges(
        &self,
        request: tonic::Request<tonic::Streaming<RouteUpdateDevaddrRangesReqV1>>,
    ) -> Result<tonic::Response<RouteDevaddrRangesResV1>, tonic::Status> {
        let mut stream = request.into_inner();

        while let Ok(Some(update)) = stream.message().await {
            match update.action() {
                ActionV1::Add => {
                    let devaddr = update.devaddr_range.expect("devaddr to update exists");
                    let added = self.storage.add_devaddr(devaddr.clone());
                    info!(added, ?devaddr, "adding devaddr");
                }
                ActionV1::Remove => {
                    let devaddr = update.devaddr_range.expect("devaddr to update exists");
                    let removed = self.storage.remove_devaddr(devaddr.clone());
                    info!(removed, ?devaddr, "removing devaddr");
                }
            }
        }

        Ok(Response::new(RouteDevaddrRangesResV1 {}))
    }

    async fn delete_devaddr_ranges(
        &self,
        request: tonic::Request<RouteDeleteDevaddrRangesReqV1>,
    ) -> Result<tonic::Response<RouteDevaddrRangesResV1>, tonic::Status> {
        let req = request.into_inner();

        self.storage.clear_devaddrs_for_route(&req.route_id);
        info!(route_id = req.route_id, "removing all devaddrs");

        Ok(Response::new(RouteDevaddrRangesResV1 {}))
    }
}
