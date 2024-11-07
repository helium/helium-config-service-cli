use helium_crypto::{Keypair, PublicKey};
use helium_lib::{
    client::{GetAnchorAccount, SolanaRpcClient},
    dao, iot_routing_manager,
};
use helium_proto::{
    services::iot_config::{
        org_client, OrgEnableReqV1, OrgEnableResV1, OrgGetReqV1, OrgListReqV1, OrgListResV1,
        OrgResV1, OrgUpdateReqV1,
    },
    Message,
};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair as SolanaKeypair, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

use crate::{
    clients::{
        error::Error,
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

    pub async fn create_helium<C: AsRef<SolanaRpcClient>>(
        &mut self,
        client: &C,
        owner: Option<Pubkey>,
        delegates: Vec<Pubkey>,
        devaddr_num_blocks: u32,
        net_id: HeliumNetId,
    ) -> Result<(Pubkey, Vec<Instruction>), Error> {
        let mut ixs = Vec::new();
        let payer = client.get_payer();
        let sub_dao_key = dao::SubDao::Iot.key();
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        let net_id_key = iot_routing_manager::net_id_key(routing_manager, u32::from(net_id.id()));
        let (organization_key, create_org_ix) = iot_routing_manager::organization::create(
            client, payer, net_id_key, authority, recipient,
        )
        .await?;

        let approve_org_ix = iot_routing_manager::organization::approve(
            client,
            authority,
            organizaion_key,
            net_id_key,
        )
        .await?;

        if !delegates.is_empty() {
            for delegate in delegates {
                let delegate_key = organization_delegate_key(&organization_key, &delegate, None);
                let delegate_acc = client
                    .as_ref()
                    .anchor_account::<OrganizationDelegateV0>(&delegate_key)
                    .await?;

                if delegate_acc.is_none() {
                    ixs.push(Self::make_init_delegate_ix(
                        payer,
                        payer,
                        organization_key,
                        delegate,
                    )?);
                }
            }
        }

        if devaddr_num_blocks > 0 {
            ixs.push(Self::make_init_devaddr_constraint_ix(
                payer,
                InitializeDevaddrConstraintArgsV0 {
                    start_addr: None,
                    num_blocks: devaddr_num_blocks,
                },
                net_id_acc.current_addr_offset,
                owner.unwrap_or(payer),
                organization_key,
                net_id_key,
                routing_manager_key,
            )?)
        }

        Ok((organization_key, ixs))
    }

    pub async fn create_roamer(
        &mut self,
        client: &SolanaClient,
        owner: Option<Pubkey>,
        delegates: Vec<Pubkey>,
        net_id: NetId,
    ) -> Result<(u64, Vec<Instruction>), Error> {
        let mut ixs = Vec::new();
        let oui: u64;
        let payer = client.get_payer();
        let iot_mint_key = Pubkey::from_str(IOT_MINT)?;
        let sub_dao_key = sub_dao_key(&iot_mint_key, None);
        let shared_merkle_key = shared_merkle_key(3, None);
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        let (net_id_key, _net_id_acc) = self
            .ensure_net_id_exists(client, net_id)
            .await
            .map_err(|_| Error::AccountNotFound("Invalid net id".to_string()))?;

        let routing_manager = client
            .anchor_account::<IotRoutingManagerV0>(&routing_manager_key)
            .await?
            .ok_or(Error::AccountNotFound(
                "Routing Manager account not found".to_string(),
            ))?;

        let organization_key =
            organization_key(&routing_manager_key, routing_manager.next_oui_id, None);
        let organization_acc = client
            .anchor_account::<OrganizationV0>(&organization_key)
            .await?;

        let (merkle_tree_key, merkle_ixs) =
            Self::get_or_create_merkle_tree(&client, &payer, shared_merkle_key).await?;

        ixs.extend(merkle_ixs);

        match organization_acc {
            Some(org) => {
                oui = org.oui;

                if !org.approved {
                    ixs.push(Self::make_approve_org_ix(
                        payer,
                        organization_key,
                        net_id_key,
                    )?);
                }
            }
            None => {
                oui = routing_manager.next_oui_id;

                ixs.push(Self::make_init_org_ix(
                    &payer.clone(),
                    routing_manager.next_oui_id,
                    net_id_key,
                    merkle_tree_key,
                    Some(payer),
                    owner.or(Some(payer)),
                )?);

                ixs.push(Self::make_approve_org_ix(
                    payer,
                    organization_key,
                    net_id_key,
                )?);
            }
        }

        // TODO (Bry): Devadder logic

        if !delegates.is_empty() {
            for delegate in delegates {
                let delegate_key = organization_delegate_key(&organization_key, &delegate, None);
                let delegate_acc = client
                    .anchor_account::<OrganizationDelegateV0>(&delegate_key)
                    .await?;

                if delegate_acc.is_none() {
                    ixs.push(Self::make_init_delegate_ix(
                        payer,
                        payer,
                        organization_key,
                        delegate,
                    )?);
                }
            }
        }

        Ok((oui, ixs))
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
        owner: Pubkey,
    ) -> Result<Instruction, Error> {
        let payer = client.get_payer();
        let (organization_key, _organization) =
            self.ensure_organization_exists(client, oui).await?;

        let ix = Self::make_update_orgainization_ix(payer, owner, organization_key)?;

        Ok(ix)
    }

    pub async fn add_delegate_key(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        delegate_key: &Pubkey,
    ) -> Result<Instruction, Error> {
        let payer = client.get_payer();
        let (organization_key, _organization) =
            self.ensure_organization_exists(client, oui).await?;

        let ix = Self::make_init_delegate_ix(payer, payer, organization_key, delegate_key.clone())?;

        Ok(ix)
    }

    pub async fn remove_delegate_key(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        delegate_key: &Pubkey,
    ) -> Result<Instruction, Error> {
        let payer = client.get_payer();
        let (organization_key, _organization) =
            self.ensure_organization_exists(client, oui).await?;

        let ix =
            Self::make_remove_delegate_ix(payer, payer, organization_key, delegate_key.clone())?;

        Ok(ix)
    }

    pub async fn add_devaddr_constraint(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        num_blocks: u32,
    ) -> Result<Instruction, Error> {
        let payer = client.get_payer();
        let (organization_key, organization) = self.ensure_organization_exists(client, oui).await?;
        let net_id = client
            .anchor_account::<NetIdV0>(&organization.net_id)
            .await?
            .ok_or(Error::AccountNotFound(
                "Net Id account not found".to_string(),
            ))?;

        let ix = Self::make_init_devaddr_constraint_ix(
            payer,
            InitializeDevaddrConstraintArgsV0 {
                start_addr: None,
                num_blocks,
            },
            net_id.current_addr_offset,
            payer,
            organization_key,
            organization.net_id,
            organization.routing_manager,
        )?;

        Ok(ix)
    }

    pub async fn remove_devaddr_constraint(
        &mut self,
        client: &SolanaClient,
        oui: u64,
        constraint: Pubkey,
    ) -> Result<Instruction, Error> {
        let payer = client.get_payer();
        let (_organization_key, organization) =
            self.ensure_organization_exists(client, oui).await?;

        let ix =
            Self::make_remove_devaddr_constraint_ix(payer, payer, organization.net_id, constraint)?;

        Ok(ix)
    }

    async fn ensure_organization_exists(
        &mut self,
        client: &SolanaClient,
        oui: u64,
    ) -> Result<(Pubkey, OrganizationV0), Error> {
        let iot_mint_key = Pubkey::from_str(IOT_MINT)?;
        let sub_dao_key = sub_dao_key(&iot_mint_key, None);
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        let organization_key = organization_key(&routing_manager_key, oui, None);

        match client
            .anchor_account::<OrganizationV0>(&organization_key)
            .await?
        {
            Some(organization) => Ok((organization_key, organization)),
            None => Err(Error::AccountNotFound(
                "Organization account not found".to_string(),
            )),
        }
    }

    async fn ensure_net_id_exists(
        &mut self,
        client: &SolanaClient,
        net_id: NetId,
    ) -> Result<(Pubkey, NetIdV0), Error> {
        let iot_mint_key = Pubkey::from_str(IOT_MINT)?;
        let sub_dao_key = sub_dao_key(&iot_mint_key, None);
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        let net_id_key = net_id_key(&routing_manager_key, net_id, None);

        match client.anchor_account::<NetIdV0>(&net_id_key).await? {
            Some(net_id) => Ok((net_id_key, net_id)),
            None => Err(Error::AccountNotFound(
                "NetId account not found".to_string(),
            )),
        }
    }
}

impl_sign!(OrgEnableReqV1, signature);
impl_sign!(OrgUpdateReqV1, signature);

impl_verify!(OrgListResV1, signature);
impl_verify!(OrgResV1, signature);
impl_verify!(OrgEnableResV1, signature);
