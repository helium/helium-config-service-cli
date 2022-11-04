use helium_config_service_cli::{
    hex_field::HexField, route::Route, Org, OrgList, Result, RouteList,
};
use helium_crypto::{Keypair, Sign};
use helium_proto::{
    services::config::{
        org_client, route_client, OrgCreateReqV1, OrgGetReqV1, OrgListReqV1, RouteCreateReqV1,
        RouteDeleteReqV1, RouteGetReqV1, RouteListReqV1, RouteUpdateReqV1,
    },
    Message,
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

    pub async fn create(&mut self, oui: u64, owner: &str, keypair: Keypair) -> Result<Org> {
        let request = OrgCreateReqV1 {
            org: Some(Org::new(oui, owner).into()),
            signature: vec![],
            timestamp: current_timestamp()?,
        };
        Ok(self
            .client
            .create(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }
}

impl RouteClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: route_client::RouteClient::connect(host.to_owned()).await?,
        })
    }

    pub async fn list(&mut self, oui: u64, owner: &str, keypair: Keypair) -> Result<RouteList> {
        let request = RouteListReqV1 {
            oui,
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        Ok(self
            .client
            .list(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }

    pub async fn get(&mut self, id: &str, owner: &str, keypair: Keypair) -> Result<Route> {
        let request = RouteGetReqV1 {
            id: id.into(),
            owner: owner.into(),
            signature: vec![],
            timestamp: current_timestamp()?,
        };
        Ok(self
            .client
            .get(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }

    pub async fn create(
        &mut self,
        net_id: HexField<6>,
        oui: u64,
        max_copies: u32,
        owner: &str,
        keypair: Keypair,
    ) -> Result<Route> {
        let request = RouteCreateReqV1 {
            oui,
            route: Some(Route::new(net_id, oui, max_copies).into()),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        Ok(self
            .client
            .create(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }

    pub async fn delete(&mut self, id: &str, owner: &str, keypair: Keypair) -> Result<Route> {
        let request = RouteDeleteReqV1 {
            id: id.into(),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        Ok(self
            .client
            .delete(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }

    pub async fn push(&mut self, route: Route, owner: &str, keypair: Keypair) -> Result<Route> {
        let request = RouteUpdateReqV1 {
            route: Some(route.inc_nonce().into()),
            owner: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        Ok(self
            .client
            .update(request.sign(keypair)?)
            .await?
            .into_inner()
            .into())
    }
}

fn current_timestamp() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

pub trait MsgSign: Message + std::clone::Clone {
    fn sign(self, keypair: Keypair) -> Result<Self>
    where
        Self: std::marker::Sized;
}

macro_rules! impl_sign {
    ($txn_type:ty, $( $sig: ident ),+ ) => {
        impl MsgSign for $txn_type {
            fn sign(self, keypair: Keypair) -> Result<Self> {
                let mut txn = self.clone();
                $(txn.$sig = vec![];)+
                let buf = txn.encode_to_vec();
                let sig = keypair.sign(&buf)?;
                $(txn.$sig = sig)+;
                Ok(txn)
            }
        }
    }
}

impl_sign!(OrgCreateReqV1, signature);
impl_sign!(RouteListReqV1, signature);
impl_sign!(RouteGetReqV1, signature);
impl_sign!(RouteCreateReqV1, signature);
impl_sign!(RouteDeleteReqV1, signature);
impl_sign!(RouteUpdateReqV1, signature);
