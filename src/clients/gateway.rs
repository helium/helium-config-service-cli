use crate::{
    clients::utils::{MsgSign, MsgVerify},
    cmds::gateway::GatewayInfo,
    impl_sign, impl_verify, Result,
};
use anyhow::anyhow;
use helium_crypto::{Keypair, PublicKey};
use helium_proto::{
    services::iot_config::{
        gateway_client, GatewayInfoReqV1, GatewayInfoResV1, GatewayLocationReqV1,
        GatewayLocationResV1,
    },
    Message,
};
use std::str::FromStr;

pub struct GatewayClient {
    client: gateway_client::GatewayClient<helium_proto::services::Channel>,
    server_pubkey: PublicKey,
}

impl GatewayClient {
    pub async fn new(host: &str, server_pubkey: &str) -> Result<Self> {
        Ok(Self {
            client: gateway_client::GatewayClient::connect(host.to_owned()).await?,
            server_pubkey: helium_crypto::PublicKey::from_str(server_pubkey)?,
        })
    }

    pub async fn location(
        &mut self,
        hotspot: &PublicKey,
        keypair: &Keypair,
    ) -> Result<GatewayLocationResV1> {
        let mut request = GatewayLocationReqV1 {
            gateway: hotspot.into(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.location(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response)
    }

    pub async fn info(&mut self, hotspot: &PublicKey, keypair: &Keypair) -> Result<GatewayInfo> {
        let mut request = GatewayInfoReqV1 {
            address: hotspot.into(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        let response = self.client.info(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        let info = response.info.ok_or_else(|| anyhow!("No hotspot found"))?;
        info.try_into()
    }
}

impl_sign!(GatewayLocationReqV1, signature);
impl_sign!(GatewayInfoReqV1, signature);

impl_verify!(GatewayLocationResV1, signature);
impl_verify!(GatewayInfoResV1, signature);
