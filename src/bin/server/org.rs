use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use async_trait::async_trait;
use helium_config_service_cli::{
    hex_field::{self, HexNetID},
    Org,
};
use helium_crypto::PublicKey;
use helium_proto::services::config::{
    org_server::Org as OrgServer, OrgCreateHeliumReqV1, OrgCreateRoamerReqV1, OrgGetReqV1,
    OrgListReqV1, OrgListResV1, OrgResV1, OrgV1,
};
use tonic::{Request, Response, Status};
use tracing::info;

type OrgMap = Arc<RwLock<HashMap<u64, Org>>>;

#[derive(Debug, Default)]
pub struct OrgService {
    orgs: OrgMap,
    next_oui: Arc<Mutex<u64>>,
}

impl OrgService {
    pub fn new(orgs: OrgMap) -> Self {
        Self {
            orgs,
            next_oui: Arc::new(Mutex::new(0)),
        }
    }

    fn next_oui(&self) -> u64 {
        let mut oui = self.next_oui.lock().expect("could not lock mutex");
        *oui += 1;
        info!(oui = oui.clone(), "next oui");
        oui.clone()
    }

    fn create_org(&self, org: Org) {
        info!(oui = org.oui, "saving org");
        let key = org.oui;
        self.orgs.write().unwrap().insert(key, org);
    }

    fn print_orgs(&self) {
        let a = self.orgs.read().unwrap();
        println!("{a:#?}");
    }

    fn get_orgs(&self) -> Vec<Org> {
        info!("getting all orgs");
        self.orgs.read().unwrap().clone().into_values().collect()
    }
}

#[async_trait]
impl OrgServer for OrgService {
    async fn list(
        &self,
        _request: Request<OrgListReqV1>,
    ) -> Result<Response<OrgListResV1>, Status> {
        self.print_orgs();

        let orgs = self.get_orgs().into_iter().map(|i| i.into()).collect();

        Ok(Response::new(OrgListResV1 { orgs }))
    }

    async fn get(&self, request: Request<OrgGetReqV1>) -> Result<Response<OrgResV1>, Status> {
        let req = request.into_inner();
        info!(oui = req.oui, "getting org");
        let org = { self.orgs.read().unwrap().get(&req.oui).map(|i| i.clone()) };
        match org {
            Some(org) => {
                info!(oui = req.oui, "found");
                Ok(Response::new(OrgResV1 {
                    org: Some(OrgV1::from(org)),
                    net_id: 0,
                    devaddr_ranges: vec![],
                }))
            }
            _ => {
                info!(oui = req.oui, "does not exist");
                Err(Status::not_found(format!("org {}", req.oui)))
            }
        }
    }

    // #[instrument]
    async fn create_helium(
        &self,
        request: tonic::Request<OrgCreateHeliumReqV1>,
    ) -> Result<tonic::Response<OrgResV1>, Status> {
        info!("creating helium org");
        let req = request.into_inner();

        let org = Org {
            oui: self.next_oui(),
            owner: PublicKey::try_from(req.owner).unwrap(),
            payer: PublicKey::try_from(req.payer).unwrap(),
            nonce: 0,
        };
        self.create_org(org.clone());
        let net_id = hex_field::net_id(0xC00053);

        Ok(Response::new(OrgResV1 {
            org: Some(OrgV1::from(org)),
            net_id: 0,
            devaddr_ranges: vec![net_id.range_start().to_range(8).into()],
        }))
    }

    async fn create_roamer(
        &self,
        request: tonic::Request<OrgCreateRoamerReqV1>,
    ) -> Result<tonic::Response<OrgResV1>, Status> {
        info!("creating roaming org");
        let req = request.into_inner();

        let org = Org {
            oui: self.next_oui(),
            owner: PublicKey::try_from(req.owner).unwrap(),
            payer: PublicKey::try_from(req.payer).unwrap(),
            nonce: 0,
        };
        self.create_org(org.clone());
        let net_id: HexNetID = req.net_id.into();

        Ok(Response::new(OrgResV1 {
            org: Some(OrgV1::from(org)),
            net_id: 0,
            devaddr_ranges: vec![net_id.full_range().into()],
        }))
    }
}
