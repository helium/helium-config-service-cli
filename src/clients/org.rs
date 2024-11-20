use helium_crypto::{Keypair, PublicKey};
use helium_lib::{
    dao,
    error::Error,
    iot_routing_manager::{
        self, devaddr_constraint,
        net_id::{self, NetIdIdentifier},
        net_id_key, orgainization_delegate,
        organization::{self, OrgIdentifier},
        routing_manager_key,
    },
};
use helium_proto::{
    services::iot_config::{
        org_client, OrgEnableReqV1, OrgEnableResV1, OrgGetReqV1, OrgListReqV1, OrgListResV1,
        OrgResV1,
    },
    Message,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use std::str::FromStr;

use crate::{
    clients::{
        utils::{current_timestamp, MsgSign, MsgVerify},
        SolanaClient,
    },
    helium_netids::HeliumNetId,
    impl_sign, impl_verify, NetId, OrgList, OrgResponse, Oui, Result,
};

pub struct OrgClient {
    client: org_client::OrgClient<helium_proto::services::Channel>,
    server_pubkey: PublicKey,
}

pub enum OrgType {
    Helium(HeliumNetId),
    Roamer(NetId),
}

impl OrgClient {
    pub async fn new(host: &str, server_pubkey: &str) -> Result<Self> {
        Ok(Self {
            client: org_client::OrgClient::connect(host.to_owned()).await?,
            server_pubkey: helium_crypto::PublicKey::from_str(server_pubkey)?,
        })
    }

    pub async fn list(&mut self) -> Result<OrgList> {
        let request = OrgListReqV1 {};
        let response = self.client.list(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response.into())
    }

    pub async fn get(&mut self, oui: Oui) -> Result<OrgResponse> {
        let request = OrgGetReqV1 { oui };
        let response = self.client.get(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(response.into())
    }

    pub async fn create_net_id(
        &mut self,
        client: &SolanaClient,
        net_id: NetId,
    ) -> Result<(Pubkey, Instruction), Error> {
        let payer = client.wallet()?;
        let (net_id_key, create_net_id_ix) = net_id::create(
            client,
            payer,
            iot_routing_manager::InitializeNetIdArgsV0 { net_id },
            Some(payer),
        )
        .await?;

        Ok((net_id_key, create_net_id_ix))
    }

    pub async fn create_org(
        &mut self,
        client: &SolanaClient,
        owner: Option<Pubkey>,
        recipient: Option<Pubkey>,
        org_type: OrgType,
    ) -> Result<(Pubkey, Instruction), Error> {
        let payer = client.wallet()?;
        let authority = owner.unwrap_or(payer);
        let sub_dao_key = dao::SubDao::Iot.key();
        let routing_manager_key = routing_manager_key(&sub_dao_key);

        let net_id_key = match org_type {
            OrgType::Helium(net_id) => net_id_key(&routing_manager_key, u32::from(net_id.id())),
            OrgType::Roamer(net_id) => {
                iot_routing_manager::net_id_key(&routing_manager_key, net_id)
            }
        };

        let (organization_key, create_org_ix) = iot_routing_manager::organization::create(
            client,
            payer,
            net_id_key,
            Some(authority),
            recipient,
        )
        .await?;

        Ok((organization_key, create_org_ix))
    }

    pub async fn approve(&mut self, client: &SolanaClient, oui: u64) -> Result<Instruction, Error> {
        let authority = client.wallet()?;
        let (organization_key, organization) =
            organization::ensure_exists(client, OrgIdentifier::Oui(oui)).await?;
        let approve_org_ix = iot_routing_manager::organization::approve(
            client,
            authority,
            organization_key,
            organization.net_id,
        )
        .await?;

        Ok(approve_org_ix)
    }

    pub async fn enable(&mut self, oui: u64, keypair: Keypair) -> Result<()> {
        let mut request = OrgEnableReqV1 {
            oui,
            timestamp: current_timestamp()?,
            signer: keypair.public_key().into(),
            signature: vec![],
        };
        request.signature = request.sign(&keypair)?;
        let response = self.client.enable(request).await?.into_inner();
        response.verify(&self.server_pubkey)?;
        Ok(())
    }

    pub async fn update_owner(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        new_authority: Pubkey,
    ) -> Result<(Pubkey, Instruction), Error> {
        let authority = client.wallet()?;
        let (organization_key, _organization) =
            organization::ensure_exists(client, OrgIdentifier::Oui(oui)).await?;
        let ix = organization::update(
            client,
            authority,
            organization_key,
            iot_routing_manager::UpdateOrganizationArgsV0 {
                new_authority: Some(new_authority),
            },
        )
        .await?;

        Ok((organization_key, ix))
    }

    pub async fn add_delegate_key(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        delegate_key: Pubkey,
    ) -> Result<Instruction, Error> {
        let payer = client.wallet()?;
        let (organization_key, _organization) =
            organization::ensure_exists(client, OrgIdentifier::Oui(oui)).await?;

        Ok(
            orgainization_delegate::create(client, payer, delegate_key, organization_key, None)
                .await?
                .1,
        )
    }

    pub async fn remove_delegate_key(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        delegate_key: Pubkey,
    ) -> Result<Instruction, Error> {
        let authority = client.wallet()?;
        let (organization_key, _organization) =
            organization::ensure_exists(client, OrgIdentifier::Oui(oui)).await?;

        Ok(
            orgainization_delegate::remove(client, authority, delegate_key, organization_key)
                .await?,
        )
    }

    pub async fn add_devaddr_constraint(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        num_blocks: u32,
        start_addr: Option<u64>,
    ) -> Result<Instruction, Error> {
        let payer = client.wallet()?;
        let (organization_key, organization) =
            organization::ensure_exists(client, OrgIdentifier::Oui(oui)).await?;

        let (net_id_key, net_id) =
            net_id::ensure_exists(client, NetIdIdentifier::Pubkey(organization.net_id)).await?;

        net_id
            .current_addr_offset
            .checked_add(num_blocks as u64 * 8)
            .ok_or(Error::Other("No Available Addrs".to_string()))?;

        Ok(devaddr_constraint::create(
            client,
            payer,
            iot_routing_manager::InitializeDevaddrConstraintArgsV0 {
                start_addr,
                num_blocks,
            },
            organization_key,
            net_id_key,
            None,
        )
        .await?
        .1)
    }

    pub async fn remove_devaddr_constraint(
        &mut self,
        client: &SolanaClient,
        devaddr_constraint_key: Pubkey,
    ) -> Result<Instruction, Error> {
        let authority = client.wallet()?;

        Ok(devaddr_constraint::remove(client, authority, devaddr_constraint_key).await?)
    }
}

impl_sign!(OrgEnableReqV1, signature);

impl_verify!(OrgListResV1, signature);
impl_verify!(OrgResV1, signature);
impl_verify!(OrgEnableResV1, signature);
