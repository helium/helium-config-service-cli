use super::{GetHotspot, PathBufKeypair};
use crate::{client, region::Region, Msg, PrettyJson, Result};
use angry_purple_tiger::AnimalName;
use helium_crypto::PublicKey;
use helium_proto::services::iot_config::{
    GatewayInfo as GatewayInfoProto, GatewayLocationResV1, GatewayMetadata as GatewayMetadataProto,
};
use serde::Serialize;
use std::str::FromStr;

pub async fn location(args: GetHotspot) -> Result<Msg> {
    let mut client = client::GatewayClient::new(&args.config_host, &args.config_pubkey).await?;
    match client
        .location(&args.hotspot, &args.keypair.to_keypair()?)
        .await
    {
        Ok(location) => {
            let location = Location::from_proto_resp(args.hotspot.to_owned(), location)?;
            Msg::ok(location.pretty_json()?)
        }
        Err(err) => Msg::err(format!(
            "failed to retrieve {} location: {}",
            &args.hotspot, err
        )),
    }
}

pub async fn info(args: GetHotspot) -> Result<Msg> {
    let mut client = client::GatewayClient::new(&args.config_host, &args.config_pubkey).await?;
    match client
        .info(&args.hotspot, &args.keypair.to_keypair()?)
        .await
    {
        Ok(info) => Msg::ok(info.pretty_json()?),
        Err(err) => Msg::err(format!(
            "failed to retrieve {} info: {}",
            &args.hotspot, err
        )),
    }
}

#[derive(Debug, Serialize)]
pub struct Location {
    name: String,
    pubkey: PublicKey,
    hex: String,
    lat: f64,
    lon: f64,
}

impl Location {
    fn from_proto_resp(
        pubkey: PublicKey,
        res: GatewayLocationResV1,
    ) -> Result<Location, h3o::error::InvalidCellIndex> {
        let hex = res.location;
        let latlng: h3o::LatLng = h3o::CellIndex::from_str(&hex)?.into();
        let name: AnimalName = pubkey.clone().into();
        Ok(Self {
            name: name.to_string(),
            pubkey,
            hex,
            lat: latlng.lat(),
            lon: latlng.lng(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct GatewayInfo {
    name: String,
    pubkey: PublicKey,
    is_full_hotspot: bool,
    metadata: Option<GatewayMetadata>,
}

#[derive(Debug, Serialize)]
pub struct GatewayMetadata {
    location: String,
    lat: f64,
    lon: f64,
    region: Region,
    gain: i32,
    elevation: i32,
}

impl TryFrom<GatewayInfoProto> for GatewayInfo {
    type Error = anyhow::Error;

    fn try_from(info: GatewayInfoProto) -> Result<Self, Self::Error> {
        let pubkey = PublicKey::try_from(info.address)?;
        let name: AnimalName = pubkey.clone().into();
        let metadata = if let Some(md) = info.metadata {
            Some(md.try_into()?)
        } else {
            None
        };
        Ok(Self {
            name: name.to_string(),
            pubkey,
            is_full_hotspot: info.is_full_hotspot,
            metadata,
        })
    }
}

impl TryFrom<GatewayMetadataProto> for GatewayMetadata {
    type Error = h3o::error::InvalidCellIndex;

    fn try_from(md: GatewayMetadataProto) -> Result<Self, Self::Error> {
        let location = md.clone().location;
        let latlng: h3o::LatLng = h3o::CellIndex::from_str(&md.location)?.into();
        Ok(Self {
            location,
            lat: latlng.lat(),
            lon: latlng.lng(),
            region: md.region().into(),
            gain: md.gain,
            elevation: md.elevation,
        })
    }
}
