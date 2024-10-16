use helium_anchor_gen::{
    anchor_lang::{InstructionData, ToAccountMetas},
    helium_entity_manager::{self, InitializeSharedMerkleArgsV0, SharedMerkleV0},
    iot_routing_manager::{
        self, typedefs::*, IotRoutingManagerV0, NetIdV0, OrganizationDelegateV0, OrganizationV0,
    },
};
use helium_crypto::{Keypair, PublicKey};
use helium_proto::{
    services::iot_config::{
        org_client, OrgEnableReqV1, OrgEnableResV1, OrgGetReqV1, OrgListReqV1, OrgListResV1,
        OrgResV1,
    },
    Message,
};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair as SolanaKeypair, signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

use super::solana::GetAnchorAccount;
use crate::{
    clients::{
        error::Error,
        utils::{current_timestamp, MsgSign, MsgVerify},
        SolanaClient,
    },
    impl_sign, impl_verify,
    solana_utils::{
        bubblegum_signer_key, dao_key, devaddr_constraint_key, entity_creator_key,
        key_to_asset_key, net_id_key, organization_delegate_key, organization_key,
        program_approval_key, routing_manager_collection_key,
        routing_manager_collection_master_edition_key, routing_manager_collection_metadata_key,
        routing_manager_key, shared_merkle_key, sub_dao_key, tree_authority_key,
        BUBBLEGUM_PROGRAM_ID, COMPRESSION_PROGRAM_ID, HNT_MINT, IOT_MINT, IOT_PRICE_FEED,
        LOG_WRAPPER_PROGRAM_ID, TM_PROGRAM_ID,
    },
    HeliumNetId, NetId, OrgList, OrgResponse, Oui, Result,
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

    async fn get_or_create_merkle_tree(
        client: &SolanaClient,
        payer: &Pubkey,
        shared_merkle_key: Pubkey,
    ) -> Result<(Pubkey, Vec<Instruction>), Error> {
        let mut ixs = Vec::new();
        let shared_merkle: Option<SharedMerkleV0> =
            client.anchor_account(&shared_merkle_key).await?;
        let merkle_tree = if let Some(existing_shared_merkle) = shared_merkle {
            existing_shared_merkle.merkle_tree
        } else {
            let merkle_tree_keypair = SolanaKeypair::new();
            let merkle_tree = merkle_tree_keypair.pubkey();

            ixs.push(Instruction {
                program_id: helium_entity_manager::ID,
                accounts: helium_entity_manager::accounts::InitializeSharedMerkleV0 {
                    payer: payer.clone(),
                    shared_merkle: shared_merkle_key,
                    merkle_tree,
                    tree_authority: tree_authority_key(&merkle_tree),
                    bubblegum_program: Pubkey::from_str(BUBBLEGUM_PROGRAM_ID)?,
                    log_wrapper: Pubkey::from_str(LOG_WRAPPER_PROGRAM_ID)?,
                    compression_program: Pubkey::from_str(COMPRESSION_PROGRAM_ID)?,
                    system_program: solana_sdk::system_program::ID,
                }
                .to_account_metas(None),
                data: helium_entity_manager::instruction::InitializeSharedMerkleV0 {
                    _args: InitializeSharedMerkleArgsV0 { proof_size: 3 },
                }
                .data(),
            });

            merkle_tree
        };

        Ok((merkle_tree, ixs))
    }

    async fn get_or_create_net_id(
        client: &SolanaClient,
        payer: &Pubkey,
        authority: Option<Pubkey>,
        net_id: u64,
    ) -> Result<(Pubkey, Vec<Instruction>), Error> {
        let mut ixs = Vec::new();
        let iot_mint = Pubkey::from_str(IOT_MINT)?;
        let sub_dao = sub_dao_key(&iot_mint, None);
        let routing_manager_key = routing_manager_key(&sub_dao, None);
        let routing_manager: IotRoutingManagerV0 = client
            .anchor_account(&routing_manager_key)
            .await?
            .ok_or(Error::AccountNotFound(
                "Routing Manager account not found".to_string(),
            ))?;

        let net_id_key = net_id_key(&routing_manager_key, net_id, None);
        let net_id_exists = client
            .anchor_account::<NetIdV0>(&net_id_key)
            .await?
            .is_some();

        if !net_id_exists {
            ixs.push(Instruction {
                program_id: iot_routing_manager::ID,
                accounts: iot_routing_manager::accounts::InitializeNetIdV0 {
                    payer: payer.clone(),
                    routing_manager: routing_manager_key,
                    net_id_authority: routing_manager.net_id_authority,
                    authority: authority.unwrap_or(payer.clone()),
                    net_id: net_id_key,
                    system_program: solana_sdk::system_program::ID,
                }
                .to_account_metas(None),
                data: iot_routing_manager::instruction::InitializeNetIdV0 {
                    _args: InitializeNetIdArgsV0 { net_id },
                }
                .data(),
            });
        }

        Ok((net_id_key, ixs))
    }

    fn make_init_org_ix(
        payer: &Pubkey,
        args: InitializeOrganizationArgsV0,
        net_id: Pubkey,
        merkle_tree: Pubkey,
        authority: Option<Pubkey>,
        recipient: Option<Pubkey>,
    ) -> Result<Instruction, Error> {
        let hnt_mint = Pubkey::from_str(HNT_MINT)?;
        let iot_mint = Pubkey::from_str(IOT_MINT)?;
        let dao = dao_key(&hnt_mint, None);
        let sub_dao = sub_dao_key(&iot_mint, None);
        let shared_merkle_key = shared_merkle_key(3, None);
        let program_approval = program_approval_key(&dao, None, None);
        let routing_manager = routing_manager_key(&sub_dao, None);
        let organization = organization_key(&routing_manager, args.oui, None);
        let collection = routing_manager_collection_key(&routing_manager, None);
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::InitializeOrganizationV0 {
                payer: payer.clone(),
                program_approval,
                routing_manager,
                net_id,
                iot_mint,
                payer_iot_account: get_associated_token_address(&payer, &iot_mint),
                iot_price_oracle: Pubkey::from_str(IOT_PRICE_FEED)?,
                authority: authority.unwrap_or(payer.clone()),
                bubblegum_signer: bubblegum_signer_key(),
                shared_merkle: shared_merkle_key,
                helium_entity_manager_program: helium_entity_manager::ID,
                dao,
                sub_dao,
                organization,
                collection,
                collection_metadata: routing_manager_collection_metadata_key(&collection),
                collection_master_edition: routing_manager_collection_master_edition_key(
                    &collection,
                ),
                entity_creator: entity_creator_key(&dao, None),
                key_to_asset: key_to_asset_key(&dao, args.oui, None),
                tree_authority: tree_authority_key(&merkle_tree),
                recipient: recipient.unwrap_or(payer.clone()),
                merkle_tree,
                bubblegum_program: Pubkey::from_str(BUBBLEGUM_PROGRAM_ID)?,
                token_program: spl_token::id(),
                token_metadata_program: Pubkey::from_str(TM_PROGRAM_ID)?,
                log_wrapper: Pubkey::from_str(LOG_WRAPPER_PROGRAM_ID)?,
                compression_program: Pubkey::from_str(COMPRESSION_PROGRAM_ID)?,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::InitializeOrganizationV0 { _args: args }.data(),
        };

        Ok(ix)
    }

    pub fn make_approve_org_ix(
        authority: Pubkey,
        organization: Pubkey,
        net_id: Pubkey,
    ) -> Result<Instruction, Error> {
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::ApproveOrganizationV0 {
                organization,
                authority,
                net_id,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::ApproveOrganizationV0 {}.data(),
        };

        Ok(ix)
    }

    pub fn make_init_delegate_ix(
        payer: Pubkey,
        authority: Pubkey,
        organization: Pubkey,
        delegate: Pubkey,
    ) -> Result<Instruction, Error> {
        let organization_delegate = organization_delegate_key(&organization, &delegate, None);
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::InitializeOrganizationDelegateV0 {
                payer,
                authority,
                organization,
                organization_delegate,
                delegate,
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::InitializeOrganizationDelegateV0 {}.data(),
        };

        Ok(ix)
    }

    pub fn make_remove_delegate_ix(
        rent_refund: Pubkey,
        authority: Pubkey,
        organization: Pubkey,
        delegate: Pubkey,
    ) -> Result<Instruction, Error> {
        let organization_delegate = organization_delegate_key(&organization, &delegate, None);
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::RemoveOrganizationDelegateV0 {
                rent_refund,
                authority,
                organization,
                organization_delegate,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::RemoveOrganizationDelegateV0 {}.data(),
        };

        Ok(ix)
    }

    pub fn make_init_devaddr_constraint_ix(
        payer: Pubkey,
        args: InitializeDevaddrConstraintArgsV0,
        current_addr_offset: u64,
        authority: Pubkey,
        organization: Pubkey,
        net_id: Pubkey,
        routing_manager: Pubkey,
    ) -> Result<Instruction, Error> {
        let iot_mint = Pubkey::from_str(IOT_MINT)?;
        let ix = {
            let mut accounts = iot_routing_manager::accounts::InitializeDevaddrConstraintV0 {
                payer: payer.clone(),
                authority,
                net_id,
                routing_manager,
                organization,
                iot_mint,
                payer_iot_account: get_associated_token_address(&payer, &iot_mint),
                iot_price_oracle: Pubkey::from_str(IOT_PRICE_FEED)?,
                devaddr_constraint: devaddr_constraint_key(
                    &organization,
                    args.start_addr.unwrap_or(current_addr_offset),
                    None,
                ),
                token_program: spl_token::id(),
                system_program: solana_sdk::system_program::ID,
            }
            .to_account_metas(None);

            for account in accounts.iter_mut() {
                if account.pubkey == net_id {
                    account.is_writable = true;
                    break;
                }
            }

            Instruction {
                program_id: iot_routing_manager::ID,
                accounts,
                data: iot_routing_manager::instruction::InitializeDevaddrConstraintV0 {
                    _args: args,
                }
                .data(),
            }
        };

        Ok(ix)
    }

    pub fn make_remove_devaddr_constraint_ix(
        rent_refund: Pubkey,
        authority: Pubkey,
        net_id: Pubkey,
        constraint: Pubkey,
    ) -> Result<Instruction, Error> {
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::RemoveDevaddrConstraintV0 {
                rent_refund,
                authority,
                net_id,
                devaddr_constraint: constraint,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::RemoveDevaddrConstraintV0 {}.data(),
        };

        Ok(ix)
    }

    pub fn make_update_orgainization_ix(
        authority: Pubkey,
        new_authority: Pubkey,
        organization: Pubkey,
    ) -> Result<Instruction, Error> {
        let ix = Instruction {
            program_id: iot_routing_manager::ID,
            accounts: iot_routing_manager::accounts::UpdateOrganizationV0 {
                authority,
                organization,
            }
            .to_account_metas(None),
            data: iot_routing_manager::instruction::UpdateOrganizationV0 {
                _args: UpdateOrganizationArgsV0 {
                    new_authority: Some(new_authority),
                },
            }
            .data(),
        };

        Ok(ix)
    }

    pub async fn create_helium(
        &mut self,
        client: &SolanaClient,
        owner: Option<Pubkey>,
        oui: u64,
        escrow_key_override: Option<String>,
        delegates: Vec<Pubkey>,
        devaddr_num_blocks: u32,
        net_id: HeliumNetId,
    ) -> Result<Vec<Instruction>, Error> {
        let mut ixs = Vec::new();
        let payer = client.get_payer();
        let iot_mint_key = Pubkey::from_str(IOT_MINT)?;
        let sub_dao_key = sub_dao_key(&iot_mint_key, None);
        let shared_merkle_key = shared_merkle_key(3, None);
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        client
            .anchor_account::<IotRoutingManagerV0>(&routing_manager_key)
            .await?
            .ok_or(Error::AccountNotFound(
                "Routing Manager account not found".to_string(),
            ))?;

        let organization_key = organization_key(&routing_manager_key, oui, None);
        let organization_acc = client
            .anchor_account::<OrganizationV0>(&organization_key)
            .await?;

        let (merkle_tree_key, merkle_ixs) =
            Self::get_or_create_merkle_tree(&client, &payer, shared_merkle_key).await?;

        let (net_id_key, net_id_ixs) =
            Self::get_or_create_net_id(&client, &payer, Some(payer), net_id as u64).await?;

        ixs.extend(merkle_ixs);
        ixs.extend(net_id_ixs);

        match organization_acc {
            Some(org) if !org.approved => {
                ixs.push(Self::make_approve_org_ix(
                    payer,
                    organization_key,
                    net_id_key,
                )?);
            }
            None => {
                ixs.push(Self::make_init_org_ix(
                    &payer.clone(),
                    InitializeOrganizationArgsV0 {
                        oui,
                        escrow_key_override,
                    },
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
            _ => {} // No action needed if organization exists and is already approved
        }

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

        if devaddr_num_blocks > 0 {
            let net_id_acc: Option<NetIdV0> = client.anchor_account(&net_id_key).await?;
            let addr_offset = net_id_acc.map_or(0, |n| n.current_addr_offset);

            // Since this fn is callable repeatedly with the same args
            // we should only create the devaddr when the net_id
            // current_addr_offset is 0
            if addr_offset == 0 {
                ixs.push(Self::make_init_devaddr_constraint_ix(
                    payer,
                    InitializeDevaddrConstraintArgsV0 {
                        start_addr: None,
                        num_blocks: devaddr_num_blocks,
                    },
                    addr_offset,
                    owner.unwrap_or(payer),
                    organization_key,
                    net_id_key,
                    routing_manager_key,
                )?)
            }
        }

        Ok(ixs)
    }

    pub async fn create_roamer(
        &mut self,
        client: &SolanaClient,
        owner: Option<Pubkey>,
        oui: u64,
        escrow_key_override: Option<String>,
        delegates: Vec<Pubkey>,
        net_id: NetId,
    ) -> Result<Vec<Instruction>, Error> {
        let mut ixs = Vec::new();
        let payer = client.get_payer();
        let iot_mint_key = Pubkey::from_str(IOT_MINT)?;
        let sub_dao_key = sub_dao_key(&iot_mint_key, None);
        let shared_merkle_key = shared_merkle_key(3, None);
        let routing_manager_key = routing_manager_key(&sub_dao_key, None);
        client
            .anchor_account::<IotRoutingManagerV0>(&routing_manager_key)
            .await?
            .ok_or(Error::AccountNotFound(
                "Routing Manager account not found".to_string(),
            ))?;

        let organization_key = organization_key(&routing_manager_key, oui, None);
        let organization_acc = client
            .anchor_account::<OrganizationV0>(&organization_key)
            .await?;

        let (merkle_tree_key, merkle_ixs) =
            Self::get_or_create_merkle_tree(&client, &payer, shared_merkle_key).await?;

        let (net_id_key, net_id_ixs) =
            Self::get_or_create_net_id(&client, &payer, Some(payer), net_id as u64).await?;

        ixs.extend(merkle_ixs);
        ixs.extend(net_id_ixs);

        match organization_acc {
            Some(org) if !org.approved => {
                ixs.push(Self::make_approve_org_ix(
                    payer,
                    organization_key,
                    net_id_key,
                )?);
            }
            None => {
                ixs.push(Self::make_init_org_ix(
                    &client.get_payer(),
                    InitializeOrganizationArgsV0 {
                        oui,
                        escrow_key_override,
                    },
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
            _ => {} // No action needed if organization exists and is already approved
        }

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

        Ok(ixs)
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
}

impl_sign!(OrgEnableReqV1, signature);

impl_verify!(OrgListResV1, signature);
impl_verify!(OrgResV1, signature);
impl_verify!(OrgEnableResV1, signature);
