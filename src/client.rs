use config_service_cli::{HexField, Org, OrgList, Result, Route, RouteList};
use helium_proto::services::config::{
    org_client, route_client, OrgGetReqV1, OrgListReqV1, RouteCreateReqV1, RouteDeleteReqV1,
    RouteGetReqV1, RouteListReqV1, RouteUpdateReqV1,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct OrgClient {
    client: org_client::OrgClient<tonic::transport::Channel>,
}
pub struct RouteClient {
    client: route_client::RouteClient<tonic::transport::Channel>,
}

impl OrgClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: org_client::OrgClient::connect(host.to_owned()).await?,
        })
    }

    pub async fn list(&mut self) -> Result<OrgList> {
        let request = OrgListReqV1 {};
        Ok(self.client.list(request).await?.into_inner().into())
    }

    pub async fn get(&mut self, oui: u64) -> Result<Org> {
        let request = OrgGetReqV1 { oui };
        Ok(self.client.get(request).await?.into_inner().into())
    }
}

impl RouteClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: route_client::RouteClient::connect(host.to_owned()).await?,
        })
    }

    pub async fn list(&mut self, oui: u64, owner: String) -> Result<RouteList> {
        let request = RouteListReqV1 {
            oui,
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: "sig".into(),
        };
        Ok(self.client.list(request).await?.into_inner().into())
    }

    pub async fn get(&mut self, id: String, owner: String) -> Result<Route> {
        let request = RouteGetReqV1 {
            id: id.into(),
            owner: owner.into(),
            signature: "sig".into(),
            timestamp: current_timestamp()?,
        };
        Ok(self.client.get(request).await?.into_inner().into())
    }

    pub async fn create(
        &mut self,
        net_id: HexField<6>,
        oui: u64,
        max_copies: u32,
        owner: String,
    ) -> Result<Route> {
        let request = RouteCreateReqV1 {
            oui,
            route: Some(Route::new(net_id, oui, max_copies).into()),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: "sig".into(),
        };
        Ok(self.client.create(request).await?.into_inner().into())
    }

    pub async fn delete(&mut self, id: String, owner: String) -> Result<Route> {
        let request = RouteDeleteReqV1 {
            id: id.into(),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: "sig".into(),
        };
        Ok(self.client.delete(request).await?.into_inner().into())
    }

    pub async fn push(&mut self, route: Route, owner: String) -> Result<Route> {
        let request = RouteUpdateReqV1 {
            route: Some(route.inc_nonce().into()),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: "sig".into(),
        };
        Ok(self.client.update(request).await?.into_inner().into())
    }
}

fn current_timestamp() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}
