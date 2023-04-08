use crate::{
    hex_field, region::Region, region_params::RegionParams, route::Route, DevaddrRange, Eui,
    KeyType, NetId, OrgList, OrgResponse, Oui, Result, RouteList, SessionKeyFilter,
};
use anyhow::anyhow;
use helium_crypto::{Keypair, PublicKey, Sign};
use helium_proto::{
    services::iot_config::{
        admin_client, org_client, route_client, session_key_filter_client, ActionV1,
        AdminAddKeyReqV1, AdminLoadRegionReqV1, AdminLoadRegionResV1, AdminRemoveKeyReqV1,
        OrgCreateHeliumReqV1, OrgCreateRoamerReqV1, OrgGetReqV1, OrgListReqV1, RouteCreateReqV1,
        RouteDeleteReqV1, RouteDevaddrRangesResV1, RouteEuisResV1, RouteGetDevaddrRangesReqV1,
        RouteGetEuisReqV1, RouteGetReqV1, RouteListReqV1, RouteUpdateDevaddrRangesReqV1,
        RouteUpdateEuisReqV1, RouteUpdateReqV1, SessionKeyFilterGetReqV1,
        SessionKeyFilterListReqV1, SessionKeyFilterUpdateReqV1, SessionKeyFilterUpdateResV1,
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

pub struct SkfClient {
    client: session_key_filter_client::SessionKeyFilterClient<tonic::transport::Channel>,
}

pub struct AdminClient {
    client: admin_client::AdminClient<tonic::transport::Channel>,
}

pub type EuiClient = RouteClient;
pub type DevaddrClient = RouteClient;

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

    pub async fn get(&mut self, oui: Oui) -> Result<OrgResponse> {
        let request = OrgGetReqV1 { oui };
        Ok(self.client.get(request).await?.into_inner().into())
    }

    pub async fn create_helium(
        &mut self,
        owner: &PublicKey,
        payer: &PublicKey,
        delegates: Vec<PublicKey>,
        devaddr_count: u64,
        keypair: &Keypair,
    ) -> Result<OrgResponse> {
        let mut request = OrgCreateHeliumReqV1 {
            owner: owner.into(),
            payer: payer.into(),
            devaddrs: devaddr_count,
            timestamp: current_timestamp()?,
            delegate_keys: delegates.iter().map(|key| key.into()).collect(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
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
        delegates: Vec<PublicKey>,
        net_id: NetId,
        keypair: Keypair,
    ) -> Result<OrgResponse> {
        let mut request = OrgCreateRoamerReqV1 {
            owner: owner.into(),
            payer: payer.into(),
            net_id,
            timestamp: current_timestamp()?,
            delegate_keys: delegates.iter().map(|key| key.into()).collect(),
            signer: keypair.public_key().into(),
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

impl DevaddrClient {
    pub async fn get_devaddrs(
        &mut self,
        route_id: &str,
        keypair: &Keypair,
    ) -> Result<Vec<DevaddrRange>> {
        let mut request = RouteGetDevaddrRangesReqV1 {
            route_id: route_id.to_string(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.get_devaddr_ranges(request).await?.into_inner();

        let mut ranges = vec![];
        while let Some(range) = stream.message().await? {
            ranges.push(range.into());
        }

        Ok(ranges)
    }

    pub async fn add_devaddrs(
        &mut self,
        devaddrs: Vec<DevaddrRange>,
        keypair: &Keypair,
    ) -> Result<RouteDevaddrRangesResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let route_devaddrs: Vec<RouteUpdateDevaddrRangesReqV1> = devaddrs
            .into_iter()
            .flat_map(|devaddr| -> Result<RouteUpdateDevaddrRangesReqV1> {
                let mut request = RouteUpdateDevaddrRangesReqV1 {
                    action: ActionV1::Add.into(),
                    timestamp,
                    signer: signer.clone(),
                    signature: vec![],
                    devaddr_range: Some(devaddr.into()),
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(route_devaddrs);
        Ok(self
            .client
            .update_devaddr_ranges(request)
            .await?
            .into_inner())
    }

    pub async fn remove_devaddrs(
        &mut self,
        devaddrs: Vec<DevaddrRange>,
        keypair: &Keypair,
    ) -> Result<RouteDevaddrRangesResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let route_devaddrs: Vec<RouteUpdateDevaddrRangesReqV1> = devaddrs
            .into_iter()
            .flat_map(|devaddr| -> Result<RouteUpdateDevaddrRangesReqV1> {
                let mut request = RouteUpdateDevaddrRangesReqV1 {
                    action: ActionV1::Remove.into(),
                    timestamp,
                    signer: signer.clone(),
                    signature: vec![],
                    devaddr_range: Some(devaddr.into()),
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(route_devaddrs);
        Ok(self
            .client
            .update_devaddr_ranges(request)
            .await?
            .into_inner())
    }

    pub async fn delete_devaddrs(&mut self, route_id: String, keypair: &Keypair) -> Result {
        let devaddrs = self.get_devaddrs(&route_id, keypair).await?;
        self.remove_devaddrs(devaddrs, keypair).await?;
        Ok(())
    }
}

impl EuiClient {
    pub async fn get_euis(&mut self, route_id: &str, keypair: &Keypair) -> Result<Vec<Eui>> {
        let mut request = RouteGetEuisReqV1 {
            route_id: route_id.to_string(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.get_euis(request).await?.into_inner();

        let mut pairs = vec![];
        while let Some(pair) = stream.message().await? {
            pairs.push(pair.into());
        }

        Ok(pairs)
    }

    pub async fn add_euis(&mut self, euis: Vec<Eui>, keypair: &Keypair) -> Result<RouteEuisResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let route_euis: Vec<RouteUpdateEuisReqV1> = euis
            .into_iter()
            .flat_map(|eui| -> Result<RouteUpdateEuisReqV1> {
                let mut request = RouteUpdateEuisReqV1 {
                    action: ActionV1::Add.into(),
                    timestamp,
                    signature: vec![],
                    signer: signer.clone(),
                    eui_pair: Some(eui.into()),
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(route_euis);
        Ok(self.client.update_euis(request).await?.into_inner())
    }

    pub async fn remove_euis(
        &mut self,
        euis: Vec<Eui>,
        keypair: &Keypair,
    ) -> Result<RouteEuisResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let route_euis: Vec<RouteUpdateEuisReqV1> = euis
            .into_iter()
            .flat_map(|eui| -> Result<RouteUpdateEuisReqV1> {
                let mut request = RouteUpdateEuisReqV1 {
                    action: ActionV1::Remove.into(),
                    timestamp,
                    signature: vec![],
                    signer: signer.clone(),
                    eui_pair: Some(eui.into()),
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(route_euis);
        Ok(self.client.update_euis(request).await?.into_inner())
    }

    pub async fn delete_euis(&mut self, route_id: String, keypair: &Keypair) -> Result {
        let euis = self.get_euis(&route_id, keypair).await?;
        self.remove_euis(euis, keypair).await?;
        Ok(())
    }
}

impl RouteClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: route_client::RouteClient::connect(host.to_owned()).await?,
        })
    }

    pub async fn list(&mut self, oui: Oui, keypair: &Keypair) -> Result<RouteList> {
        let mut request = RouteListReqV1 {
            oui,
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.list(request).await?.into_inner().into())
    }

    pub async fn get(&mut self, id: &str, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteGetReqV1 {
            id: id.into(),
            signature: vec![],
            signer: keypair.public_key().into(),
            timestamp: current_timestamp()?,
        };
        request.signature = request.sign(keypair)?;
        self.client
            .get(request)
            .await?
            .into_inner()
            .route
            .map(Route::from)
            .ok_or(anyhow!("Route get failed"))
    }

    pub async fn create_route(&mut self, route: Route, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteCreateReqV1 {
            oui: route.oui,
            route: Some(route.into()),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client
            .create(request)
            .await?
            .into_inner()
            .route
            .map(Route::from)
            .ok_or(anyhow!("Route create failed"))
    }

    pub async fn delete(&mut self, id: &str, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteDeleteReqV1 {
            id: id.into(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client
            .delete(request)
            .await?
            .into_inner()
            .route
            .map(Route::from)
            .ok_or(anyhow!("Route delete failed"))
    }

    pub async fn push(&mut self, route: Route, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteUpdateReqV1 {
            route: Some(route.into()),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client
            .update(request)
            .await?
            .into_inner()
            .route
            .map(Route::from)
            .ok_or(anyhow!("Route update push failed"))
    }
}

impl SkfClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: session_key_filter_client::SessionKeyFilterClient::connect(host.to_owned())
                .await?,
        })
    }

    pub async fn list_filters(
        &mut self,
        oui: Oui,
        keypair: &Keypair,
    ) -> Result<Vec<SessionKeyFilter>> {
        let mut request = SessionKeyFilterListReqV1 {
            oui,
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.list(request).await?.into_inner();

        let mut filters = vec![];
        while let Some(filter) = stream.message().await? {
            filters.push(filter.into());
        }

        Ok(filters)
    }

    pub async fn get_filters(
        &mut self,
        oui: Oui,
        devaddr: hex_field::HexDevAddr,
        keypair: &Keypair,
    ) -> Result<Vec<SessionKeyFilter>> {
        let mut request = SessionKeyFilterGetReqV1 {
            oui,
            devaddr: devaddr.into(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.get(request).await?.into_inner();

        let mut filters = vec![];
        while let Some(filter) = stream.message().await? {
            filters.push(filter.into());
        }
        Ok(filters)
    }

    pub async fn add_filters(
        &mut self,
        filters: Vec<SessionKeyFilter>,
        keypair: &Keypair,
    ) -> Result<SessionKeyFilterUpdateResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let filters: Vec<SessionKeyFilterUpdateReqV1> = filters
            .into_iter()
            .flat_map(|filter| -> Result<SessionKeyFilterUpdateReqV1> {
                let mut request = SessionKeyFilterUpdateReqV1 {
                    action: ActionV1::Add.into(),
                    filter: Some(filter.into()),
                    timestamp,
                    signer: signer.clone(),
                    signature: vec![],
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(filters);
        Ok(self.client.update(request).await?.into_inner())
    }

    pub async fn remove_filters(
        &mut self,
        filters: Vec<SessionKeyFilter>,
        keypair: &Keypair,
    ) -> Result<SessionKeyFilterUpdateResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let filters: Vec<SessionKeyFilterUpdateReqV1> = filters
            .into_iter()
            .flat_map(|filter| -> Result<SessionKeyFilterUpdateReqV1> {
                let mut request = SessionKeyFilterUpdateReqV1 {
                    action: ActionV1::Remove.into(),
                    filter: Some(filter.into()),
                    timestamp,
                    signer: signer.clone(),
                    signature: vec![],
                };
                request.signature = request.sign(keypair)?;
                Ok(request)
            })
            .collect();
        let request = futures::stream::iter(filters);
        Ok(self.client.update(request).await?.into_inner())
    }
}

impl AdminClient {
    pub async fn new(host: &str) -> Result<Self> {
        Ok(Self {
            client: admin_client::AdminClient::connect(host.to_owned()).await?,
        })
    }

    pub async fn add_key(
        &mut self,
        pubkey: &PublicKey,
        key_type: KeyType,
        keypair: &Keypair,
    ) -> Result {
        let mut request = AdminAddKeyReqV1 {
            pubkey: pubkey.into(),
            key_type: key_type.into(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client.add_key(request).await?;
        Ok(())
    }

    pub async fn remove_key(&mut self, pubkey: &PublicKey, keypair: &Keypair) -> Result {
        let mut request = AdminRemoveKeyReqV1 {
            pubkey: pubkey.into(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client.remove_key(request).await?;
        Ok(())
    }

    pub async fn load_region(
        &mut self,
        region: Region,
        params: RegionParams,
        indexes: Vec<u8>,
        keypair: &Keypair,
    ) -> Result<AdminLoadRegionResV1> {
        let mut request = AdminLoadRegionReqV1 {
            region: region.into(),
            params: Some(params.into()),
            hex_indexes: indexes,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        Ok(self.client.load_region(request).await?.into_inner())
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
impl_sign!(RouteUpdateDevaddrRangesReqV1, signature);
impl_sign!(RouteGetEuisReqV1, signature);
impl_sign!(RouteUpdateEuisReqV1, signature);
impl_sign!(RouteGetDevaddrRangesReqV1, signature);
impl_sign!(SessionKeyFilterListReqV1, signature);
impl_sign!(SessionKeyFilterGetReqV1, signature);
impl_sign!(SessionKeyFilterUpdateReqV1, signature);
impl_sign!(OrgCreateHeliumReqV1, signature);
impl_sign!(OrgCreateRoamerReqV1, signature);
impl_sign!(AdminLoadRegionReqV1, signature);
impl_sign!(AdminAddKeyReqV1, signature);
impl_sign!(AdminRemoveKeyReqV1, signature);
