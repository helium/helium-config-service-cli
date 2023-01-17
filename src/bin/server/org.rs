use std::sync::Arc;

use async_trait::async_trait;
use helium_config_service_cli::{
    hex_field::{self, HexNetID},
    Org,
};
use helium_crypto::PublicKey;
use helium_proto::services::iot_config::{
    org_server::Org as OrgServer, OrgCreateHeliumReqV1, OrgCreateRoamerReqV1, OrgDisableReqV1,
    OrgDisableResV1, OrgGetReqV1, OrgListReqV1, OrgListResV1, OrgResV1, OrgV1,
};
use tonic::{Request, Response, Status};
use tracing::info;

use crate::storage::{OrgStorage, Storage};

#[derive(Debug)]
pub struct OrgService {
    storage: Arc<Storage>,
}

impl OrgService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl OrgServer for OrgService {
    async fn disable(
        &self,
        _request: Request<OrgDisableReqV1>,
    ) -> Result<Response<OrgDisableResV1>, Status> {
        Err(Status::unimplemented("cannot disable yet"))
    }

    async fn list(
        &self,
        _request: Request<OrgListReqV1>,
    ) -> Result<Response<OrgListResV1>, Status> {
        let orgs = self
            .storage
            .get_orgs()
            .into_iter()
            .map(|o| o.into())
            .collect();

        Ok(Response::new(OrgListResV1 { orgs }))
    }

    async fn get(&self, request: Request<OrgGetReqV1>) -> Result<Response<OrgResV1>, Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting org");

        match self.storage.get_org(req.oui) {
            Some(org) => {
                info!(oui = req.oui, "found");
                Ok(Response::new(OrgResV1 {
                    org: Some(org.into()),
                    net_id: 0,
                    devaddr_constraints: vec![],
                }))
            }
            _ => {
                info!(oui = req.oui, "does not exist");
                Err(Status::not_found(format!("org {}", req.oui)))
            }
        }
    }

    async fn create_helium(
        &self,
        request: tonic::Request<OrgCreateHeliumReqV1>,
    ) -> Result<tonic::Response<OrgResV1>, Status> {
        info!("creating helium org");
        let req = request.into_inner();

        let org = Org {
            oui: self.storage.next_oui(),
            owner: PublicKey::try_from(req.owner).unwrap(),
            payer: PublicKey::try_from(req.payer).unwrap(),
            delegate_keys: vec![],
        };

        let net_id = hex_field::net_id(0xC00053);
        let devaddr_constraint = net_id.range_start().to_range(8);
        self.storage
            .create_helium_org(org.clone(), devaddr_constraint.clone());

        Ok(Response::new(OrgResV1 {
            org: Some(OrgV1::from(org)),
            net_id: net_id.into(),
            devaddr_constraints: vec![devaddr_constraint.into()],
        }))
    }

    async fn create_roamer(
        &self,
        request: tonic::Request<OrgCreateRoamerReqV1>,
    ) -> Result<tonic::Response<OrgResV1>, Status> {
        info!("creating roaming org");
        let req = request.into_inner();

        let org = Org {
            oui: self.storage.next_oui(),
            owner: PublicKey::try_from(req.owner).unwrap(),
            payer: PublicKey::try_from(req.payer).unwrap(),
            delegate_keys: vec![],
        };

        let net_id: HexNetID = req.net_id.into();
        let devaddr_constraint = net_id.full_range();
        self.storage
            .create_roamer_org(org.clone(), devaddr_constraint.clone());

        Ok(Response::new(OrgResV1 {
            org: Some(OrgV1::from(org)),
            net_id: net_id.into(),
            devaddr_constraints: vec![devaddr_constraint.into()],
        }))
    }
}
