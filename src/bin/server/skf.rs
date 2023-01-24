use crate::storage::{SkfStorage, Storage};
use helium_config_service_cli::Result;
use helium_proto::services::iot_config::{
    session_key_filter_server, ActionV1, SessionKeyFilterGetReqV1, SessionKeyFilterListReqV1,
    SessionKeyFilterStreamReqV1, SessionKeyFilterStreamResV1, SessionKeyFilterUpdateReqV1,
    SessionKeyFilterUpdateResV1, SessionKeyFilterV1,
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
    type listStream = ReceiverStream<Result<SessionKeyFilterV1, Status>>;
    async fn list(
        &self,
        request: tonic::Request<SessionKeyFilterListReqV1>,
    ) -> Result<tonic::Response<Self::listStream>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting filters");

        let (tx, rx) = tokio::sync::mpsc::channel(50);

        match self.storage.get_filters_for_oui(req.oui) {
            Ok(filters) => {
                tokio::spawn(async move {
                    for filter in filters {
                        tx.send(Ok(filter.into()))
                            .await
                            .expect("session key filter sent")
                    }
                });
            }
            Err(e) => return Err(Status::not_found(format!("no filters: {e}"))),
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type getStream = ReceiverStream<Result<SessionKeyFilterV1, Status>>;
    async fn get(
        &self,
        request: tonic::Request<SessionKeyFilterGetReqV1>,
    ) -> Result<tonic::Response<Self::getStream>, tonic::Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting filter");

        let (tx, rx) = tokio::sync::mpsc::channel(50);
        match self
            .storage
            .get_filters_for_devaddr(req.oui, req.devaddr.into())
        {
            Ok(filters) => {
                tokio::spawn(async move {
                    for filter in filters {
                        tx.send(Ok(filter.into()))
                            .await
                            .expect("session key filter sent")
                    }
                });
            }
            Err(e) => return Err(Status::not_found(format!("filter not found: {e}"))),
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn update(
        &self,
        request: tonic::Request<tonic::Streaming<SessionKeyFilterUpdateReqV1>>,
    ) -> Result<tonic::Response<SessionKeyFilterUpdateResV1>, tonic::Status> {
        let mut stream = request.into_inner();

        while let Ok(Some(update)) = stream.message().await {
            match update.action() {
                ActionV1::Add => {
                    let filter = update.filter.expect("filter to update exists");
                    let added = self.storage.add_filter(filter.clone().into());
                    info!(added, ?filter, "adding skf");
                }
                ActionV1::Remove => {
                    let filter = update.filter.expect("filter to udpate exists");
                    let removed = self.storage.remove_filter(filter.clone().into());
                    info!(removed, ?filter, "removing skf");
                }
            }
        }

        Ok(Response::new(SessionKeyFilterUpdateResV1 {}))
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
                if (tx.send(Ok(update)).await).is_err() {
                    break;
                }
            }
            info!("disconnected");
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
