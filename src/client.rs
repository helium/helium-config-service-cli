use crate::{hex_field, route::Route, Eui, OrgList, OrgResponse, Result, RouteList};
use helium_crypto::{Keypair, PublicKey, Sign};
use helium_proto::{
    services::config::{
        org_client, route_client, OrgCreateHeliumReqV1, OrgCreateRoamerReqV1, OrgGetReqV1,
        OrgListReqV1, RouteCreateReqV1, RouteDeleteReqV1, RouteEuisActionV1, RouteEuisReqV1,
        RouteEuisResV1, RouteGetReqV1, RouteListReqV1, RouteUpdateReqV1,
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

    pub async fn get(&mut self, oui: u64) -> Result<OrgResponse> {
        let request = OrgGetReqV1 { oui };
        Ok(self.client.get(request).await?.into_inner().into())
    }

    pub async fn create_helium(
        &mut self,
        owner: &PublicKey,
        payer: &PublicKey,
        devaddr_count: u64,
        keypair: Keypair,
    ) -> Result<OrgResponse> {
        let mut request = OrgCreateHeliumReqV1 {
            owner: owner.into(),
            payer: payer.into(),
            devaddrs: devaddr_count,
            timestamp: current_timestamp()?,
            signer: owner.into(),
            signature: vec![],
        };
        request.signature = request.sign(&keypair)?;
        Ok(self
            .client
            .create_helium(request)
            .await?
            .into_inner()
            .into())
    }

    pub async fn create_roamer(
        &mut self,
        owner: &PublicKey,
        payer: &PublicKey,
        net_id: u64,
        keypair: Keypair,
    ) -> Result<OrgResponse> {
        let mut request = OrgCreateRoamerReqV1 {
            owner: owner.into(),
            payer: payer.into(),
            net_id,
            timestamp: current_timestamp()?,
            signer: owner.into(),
            signature: vec![],
        };
        request.signature = request.sign(&keypair)?;
        Ok(self
            .client
            .create_roamer(request)
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

    pub async fn list(
        &mut self,
        oui: u64,
        owner: &PublicKey,
        keypair: &Keypair,
    ) -> Result<RouteList> {
        let mut request = RouteListReqV1 {
            oui,
            signer: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.list(request).await?.into_inner().into())
    }

    pub async fn get(&mut self, id: &str, owner: &PublicKey, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteGetReqV1 {
            id: id.into(),
            signer: owner.into(),
            signature: vec![],
            timestamp: current_timestamp()?,
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.get(request).await?.into_inner().into())
    }

    pub async fn create(
        &mut self,
        net_id: hex_field::HexNetID,
        oui: u64,
        max_copies: u32,
        owner: &PublicKey,
        keypair: &Keypair,
    ) -> Result<Route> {
        let mut request = RouteCreateReqV1 {
            oui,
            route: Some(Route::new(net_id, oui, max_copies).into()),
            signer: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.create(request).await?.into_inner().into())
    }

    pub async fn create_route(
        &mut self,
        route: Route,
        owner: &PublicKey,
        keypair: &Keypair,
    ) -> Result<Route> {
        let mut request = RouteCreateReqV1 {
            oui: route.oui,
            route: Some(route.into()),
            signer: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.create(request).await?.into_inner().into())
    }

    pub async fn delete(
        &mut self,
        id: &str,
        owner: &PublicKey,
        keypair: &Keypair,
    ) -> Result<Route> {
        let mut request = RouteDeleteReqV1 {
            id: id.into(),
            signer: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.delete(request).await?.into_inner().into())
    }

    pub async fn push(
        &mut self,
        route: Route,
        owner: &PublicKey,
        keypair: &Keypair,
    ) -> Result<Route> {
        let mut request = RouteUpdateReqV1 {
            route: Some(route.inc_nonce().into()),
            signer: owner.into(),
            timestamp: current_timestamp()?,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.update(request).await?.into_inner().into())
    }

    pub async fn euis(
        &mut self,
        route_id: String,
        euis: Vec<Eui>,
        keypair: &Keypair,
    ) -> Result<RouteEuisResV1> {
        let mut request = RouteEuisReqV1 {
            action: RouteEuisActionV1::Add.into(),
            euis: euis.into_iter().map(|e| e.into()).collect(),
            id: route_id,
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.euis(request).await?.into_inner().into())
    }
}

fn current_timestamp() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

pub trait MsgSign: Message + std::clone::Clone {
    fn sign(&self, keypair: &Keypair) -> Result<Vec<u8>>
    where
        Self: std::marker::Sized;
}

macro_rules! impl_sign {
    ($txn_type:ty, $( $sig: ident ),+ ) => {
        impl MsgSign for $txn_type {
            fn sign(&self, keypair: &Keypair) -> Result<Vec<u8>> {
                let mut txn = self.clone();
                $(txn.$sig = vec![];)+
                Ok(keypair.sign(&txn.encode_to_vec())?)
            }
        }
    }
}

impl_sign!(RouteListReqV1, signature);
impl_sign!(RouteGetReqV1, signature);
impl_sign!(RouteCreateReqV1, signature);
impl_sign!(RouteDeleteReqV1, signature);
impl_sign!(RouteUpdateReqV1, signature);
impl_sign!(RouteEuisReqV1, signature);
impl_sign!(OrgCreateHeliumReqV1, signature);
impl_sign!(OrgCreateRoamerReqV1, signature);
