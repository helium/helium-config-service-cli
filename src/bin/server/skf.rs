use crate::storage::{SessionKeyFilter, SkfStorage, Storage};
use helium_config_service_cli::Result;
use helium_proto::services::config::{
    session_key_filter_server, SessionKeyFilterCreateReqV1, SessionKeyFilterDeleteReqV1,
    SessionKeyFilterGetReqV1, SessionKeyFilterListReqV1, SessionKeyFilterListResV1,
    SessionKeyFilterStreamReqV1, SessionKeyFilterStreamResV1, SessionKeyFilterUpdateReqV1,
    SessionKeyFilterV1,
};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::info;

#[derive(Debug)]
pub struct SKFService {
    storage: Arc<Storage>,
}

impl SKFService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
}

#[tonic::async_trait]
impl session_key_filter_server::SessionKeyFilter for SKFService {
    async fn list(
        &self,
        request: tonic::Request<SessionKeyFilterListReqV1>,
    ) -> Result<tonic::Response<SessionKeyFilterListResV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting filters");
        match self.storage.get_filters(req.oui) {
            Ok(filters) => Ok(Response::new(SessionKeyFilterListResV1 {
                filters: filters.iter().map(|f| f.to_owned().into()).collect(),
            })),
            Err(e) => Err(Status::not_found(format!("no filters: {e}"))),
        }
    }

    async fn get(
        &self,
        request: tonic::Request<SessionKeyFilterGetReqV1>,
    ) -> Result<tonic::Response<SessionKeyFilterV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting filter");
        match self.storage.get_filter(req.oui) {
            Ok(filter) => Ok(Response::new(filter.into())),
            Err(e) => Err(Status::not_found(format!("filter not found: {e}"))),
        }
    }

    async fn create(
        &self,
        request: tonic::Request<SessionKeyFilterCreateReqV1>,
    ) -> Result<tonic::Response<SessionKeyFilterV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "creating filter");
        let filter: SessionKeyFilter = req.filter.expect("filter to create").into();
        match self.storage.create_filter(req.oui, filter) {
            Ok(filter) => Ok(Response::new(filter.into())),
            Err(e) => Err(Status::not_found(format!("could not create: {e}"))),
        }
    }

    async fn update(
        &self,
        request: tonic::Request<SessionKeyFilterUpdateReqV1>,
    ) -> Result<tonic::Response<SessionKeyFilterV1>, tonic::Status> {
        let req = request.into_inner();
        info!("updating filter");

        let filter: SessionKeyFilter = req.filter.expect("filter to update").into();
        match self.storage.update_filter(req.oui, filter) {
            Ok(filter) => Ok(Response::new(filter.into())),
            Err(e) => Err(Status::not_found(format!("filter not found: {e}"))),
        }
    }

    async fn delete(
        &self,
        request: tonic::Request<SessionKeyFilterDeleteReqV1>,
    ) -> Result<tonic::Response<SessionKeyFilterV1>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "deleting filter");

        match self.storage.delete_filter(req.oui) {
            Ok(filter) => Ok(Response::new(filter.into())),
            Err(e) => Err(Status::not_found(format!("filter not found: {e}"))),
        }
    }

    type streamStream = ReceiverStream<Result<SessionKeyFilterStreamResV1, Status>>;

    async fn stream(
        &self,
        _request: tonic::Request<SessionKeyFilterStreamReqV1>,
    ) -> Result<tonic::Response<Self::streamStream>, tonic::Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut updates = self.storage.subscribe_to_filters();

        info!("connected");

        tokio::spawn(async move {
            while let Ok(update) = updates.recv().await {
                info!("filter updated");
                if let Err(_) = tx.send(Ok(update)).await {
                    break;
                }
            }
            info!("disconnected");
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
