use super::{GetLocation, PathBufKeypair};
use crate::{client, Msg, PrettyJson, Result};
use angry_purple_tiger::AnimalName;
use h3ron::ToCoordinate;
use helium_crypto::PublicKey;
use helium_proto::services::iot_config::GatewayLocationResV1;
use serde::Serialize;
use std::str::FromStr;

pub async fn location(args: GetLocation) -> Result<Msg> {
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
    ) -> Result<Location, h3ron::Error> {
        let hex = res.location;
        let (lat, lon) = h3ron::H3Cell::from_str(&hex)?.to_coordinate()?.x_y();
        let name: AnimalName = pubkey.clone().into();
        Ok(Self {
            name: name.to_string(),
            pubkey,
            hex,
            lat,
            lon,
        })
    }
}
