use crate::{
    clients::utils::{MsgSign, MsgVerify},
    impl_sign, impl_verify,
    region::Region,
    region_params::RegionParams,
    KeyType, Result,
};
use helium_crypto::{Keypair, PublicKey};
use helium_proto::{
    services::iot_config::{
        admin_client, AdminAddKeyReqV1, AdminKeyResV1, AdminLoadRegionReqV1, AdminLoadRegionResV1,
        AdminRemoveKeyReqV1,
    },
    Message,
};
use std::str::FromStr;

pub struct AdminClient {
    client: admin_client::AdminClient<helium_proto::services::Channel>,
    server_pubkey: PublicKey,
}

impl AdminClient {
    pub async fn new(host: &str, server_pubkey: &str) -> Result<Self> {
        Ok(Self {
            client: admin_client::AdminClient::connect(host.to_owned()).await?,
            server_pubkey: helium_crypto::PublicKey::from_str(server_pubkey)?,
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
        self.client
            .add_key(request)
            .await?
            .into_inner()
            .verify(&self.server_pubkey)
    }

    pub async fn remove_key(&mut self, pubkey: &PublicKey, keypair: &Keypair) -> Result {
        let mut request = AdminRemoveKeyReqV1 {
            pubkey: pubkey.into(),
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client
            .remove_key(request)
            .await?
            .into_inner()
            .verify(&self.server_pubkey)
    }

    pub async fn load_region(
        &mut self,
        region: Region,
        params: RegionParams,
        indexes: Vec<u8>,
        keypair: &Keypair,
    ) -> Result {
        let mut request = AdminLoadRegionReqV1 {
            region: region.into(),
            params: Some(params.into()),
            hex_indexes: indexes,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(keypair)?;
        self.client
            .load_region(request)
            .await?
            .into_inner()
            .verify(&self.server_pubkey)
    }
}

impl_sign!(AdminAddKeyReqV1, signature);
impl_sign!(AdminRemoveKeyReqV1, signature);
impl_sign!(AdminLoadRegionReqV1, signature);

impl_verify!(AdminKeyResV1, signature);
impl_verify!(AdminLoadRegionResV1, signature);
