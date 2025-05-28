use crate::{
    clients::utils::{current_timestamp, MsgSign, MsgVerify},
    hex_field, impl_sign, impl_verify,
    route::Route,
    DevaddrRange, Eui, Oui, Result, RouteList, Skf, SkfUpdate,
};
use anyhow::anyhow;
use helium_crypto::{Keypair, PublicKey};
use helium_proto::{
    services::iot_config::{
        route_client, route_skf_update_req_v1::RouteSkfUpdateV1, ActionV1, RouteCreateReqV1,
        RouteDeleteReqV1, RouteDevaddrRangesResV1, RouteEuisResV1, RouteGetDevaddrRangesReqV1,
        RouteGetEuisReqV1, RouteGetReqV1, RouteListReqV1, RouteListResV1, RouteResV1,
        RouteSkfGetReqV1, RouteSkfListReqV1, RouteSkfUpdateReqV1, RouteSkfUpdateResV1,
        RouteUpdateDevaddrRangesReqV1, RouteUpdateEuisReqV1, RouteUpdateReqV1,
    },
    Message,
};
use std::str::FromStr;

pub type EuiClient = RouteClient;
pub type DevaddrClient = RouteClient;
pub type SkfClient = RouteClient;

pub struct RouteClient {
    client: route_client::RouteClient<helium_proto::services::Channel>,
    server_pubkey: PublicKey,
}

impl RouteClient {
    pub async fn new(host: &str, server_pubkey: &str) -> Result<Self> {
        Ok(Self {
            client: route_client::RouteClient::connect(host.to_owned()).await?,
            server_pubkey: helium_crypto::PublicKey::from_str(server_pubkey)?,
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
        let response = self.client.list(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response.into())
    }

    pub async fn get(&mut self, id: &str, keypair: &Keypair) -> Result<Route> {
        let mut request = RouteGetReqV1 {
            id: id.into(),
            signature: vec![],
            signer: keypair.public_key().into(),
            timestamp: current_timestamp()?,
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.get(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        response
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
        let response = self.client.create(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        response
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
        let response = self.client.delete(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        response
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
        let response = self.client.update(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        response
            .route
            .map(Route::from)
            .ok_or(anyhow!("Route update push failed"))
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
        let response = self.client.update_euis(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
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
        let response = self.client.update_euis(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
    }

    pub async fn delete_euis(&mut self, route_id: String, keypair: &Keypair) -> Result {
        let euis = self.get_euis(&route_id, keypair).await?;
        self.remove_euis(euis, keypair).await?;
        Ok(())
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
        let response = self
            .client
            .update_devaddr_ranges(request)
            .await?
            .into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
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
        let response = self
            .client
            .update_devaddr_ranges(request)
            .await?
            .into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
    }

    pub async fn delete_devaddrs(&mut self, route_id: String, keypair: &Keypair) -> Result {
        let devaddrs = self.get_devaddrs(&route_id, keypair).await?;
        self.remove_devaddrs(devaddrs, keypair).await?;
        Ok(())
    }
}

impl SkfClient {
    pub async fn list_filters(&mut self, route_id: &str, keypair: &Keypair) -> Result<Vec<Skf>> {
        let mut request = RouteSkfListReqV1 {
            route_id: route_id.to_string(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.list_skfs(request).await?.into_inner();

        let mut filters = vec![];
        while let Some(filter) = stream.message().await? {
            filters.push(filter.into());
        }

        Ok(filters)
    }

    pub async fn get_filters(
        &mut self,
        route_id: &str,
        devaddr: hex_field::HexDevAddr,
        keypair: &Keypair,
    ) -> Result<Vec<Skf>> {
        let mut request = RouteSkfGetReqV1 {
            route_id: route_id.to_string(),
            devaddr: devaddr.into(),
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let mut stream = self.client.get_skfs(request).await?.into_inner();

        let mut filters = vec![];
        while let Some(filter) = stream.message().await? {
            filters.push(filter.into());
        }
        Ok(filters)
    }

    pub async fn add_filter(
        &mut self,
        filter: Skf,
        keypair: &Keypair,
    ) -> Result<RouteSkfUpdateResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let add_filter = RouteSkfUpdateV1 {
            devaddr: filter.devaddr.into(),
            session_key: filter.session_key,
            action: ActionV1::Add.into(),
            max_copies: filter.max_copies.unwrap_or(1),
        };
        let mut request = RouteSkfUpdateReqV1 {
            route_id: filter.route_id,
            updates: vec![add_filter],
            timestamp,
            signer,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.update_skfs(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
    }

    pub async fn remove_filter(
        &mut self,
        filter: Skf,
        keypair: &Keypair,
    ) -> Result<RouteSkfUpdateResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let remove_filter = RouteSkfUpdateV1 {
            devaddr: filter.devaddr.into(),
            session_key: filter.session_key,
            action: ActionV1::Remove.into(),
            max_copies: 0,
        };
        let mut request = RouteSkfUpdateReqV1 {
            route_id: filter.route_id,
            updates: vec![remove_filter],
            timestamp,
            signer,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.update_skfs(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
    }

    pub async fn delete_filters(&mut self, route_id: String, keypair: &Keypair) -> Result {
        let skfs = self.list_filters(&route_id, keypair).await?;
        let total = skfs.len() / 100;
        for (idx, chunk) in skfs.chunks(100).enumerate() {
            let mut request = RouteSkfUpdateReqV1 {
                route_id: route_id.clone(),
                updates: chunk
                    .iter()
                    .map(|skf| RouteSkfUpdateV1 {
                        devaddr: skf.devaddr.into(),
                        session_key: skf.session_key.to_owned(),
                        action: ActionV1::Remove.into(),
                        max_copies: 0,
                    })
                    .collect(),
                timestamp: current_timestamp()?,
                signer: keypair.public_key().into(),
                signature: vec![],
            };
            request.signature = request.sign(keypair)?;
            let response = self.client.update_skfs(request).await?.into_inner();
            response.verify(&self.server_pubkey)?;
            println!("Removed page: {idx}/{total}");
        }

        Ok(())
    }

    pub async fn update_filters(
        &mut self,
        route_id: &str,
        updates: Vec<SkfUpdate>,
        keypair: &Keypair,
    ) -> Result<RouteSkfUpdateResV1> {
        let timestamp = current_timestamp()?;
        let signer: Vec<u8> = keypair.public_key().into();
        let mut request = RouteSkfUpdateReqV1 {
            route_id: route_id.to_string(),
            updates: updates.into_iter().map(RouteSkfUpdateV1::from).collect(),
            timestamp,
            signer,
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.update_skfs(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
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
impl_sign!(RouteSkfListReqV1, signature);
impl_sign!(RouteSkfGetReqV1, signature);
impl_sign!(RouteSkfUpdateReqV1, signature);

impl_verify!(RouteDevaddrRangesResV1, signature);
impl_verify!(RouteEuisResV1, signature);
impl_verify!(RouteListResV1, signature);
impl_verify!(RouteResV1, signature);
impl_verify!(RouteSkfUpdateResV1, signature);
