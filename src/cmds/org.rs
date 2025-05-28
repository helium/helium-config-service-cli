use super::{
    ApproveOrg, CreateHelium, CreateNetId, CreateRoaming, DevaddrUpdateConstraint, DisableOrg,
    EnableOrg, GetOrg, ListOrgs, OrgUpdateKey, PathBufKeypair,
};

use crate::{
    clients::{self, OrgSolanaOperations, OrgType},
    helium_netids, Msg, PrettyJson, Result,
};

use helium_lib::iot::{
    net_id::{self, NetIdIdentifier},
    organization::{self, OrgIdentifier},
};

use std::sync::Arc;

pub async fn create_solana_client(
    solana_url: &str,
    keypair: helium_crypto::Keypair,
) -> Result<helium_lib::client::SolanaClient> {
    let keypair = helium_lib::keypair::Keypair::try_from(keypair)?;
    helium_lib::client::SolanaClient::new(solana_url, Some(Arc::new(keypair))).map_err(|e| e.into())
}

pub async fn list_orgs(args: ListOrgs) -> Result<Msg> {
    let mut client = clients::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
    let org = client.list().await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn get_org(args: GetOrg) -> Result<Msg> {
    let mut client = clients::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
    let org = client.get(args.oui).await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn create_net_id(args: CreateNetId) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;

        let (_, ix) =
            OrgSolanaOperations::create_net_id(&solana_client, args.net_id.into()).await?;

        solana_client.send_instruction(ix, &[], None).await?;

        let (_, _net_id) =
            net_id::ensure_exists(&solana_client, NetIdIdentifier::Id(args.net_id.into())).await?;

        return Msg::ok(format!("== NetId Created: {id} ==", id = args.net_id));
    }

    Msg::dry_run(format!("Create NetId {:?}", args.net_id,))
}

pub async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let netid_field = helium_netids::HeliumNetId::from(args.net_id);
        let (organization_key, ix) = OrgSolanaOperations::create_org(
            &solana_client,
            args.owner.clone(),
            args.owner.clone(),
            OrgType::Helium(netid_field),
        )
        .await?;

        solana_client.send_instruction(ix, &[], {}).await?;

        let (_, organization) =
            organization::ensure_exists(&solana_client, OrgIdentifier::Pubkey(organization_key))
                .await?;

        return Msg::ok(format!(
            "== Helium Organization Created: {oui} ==\n== Call `org get --oui {oui} to see its details` ==",
            oui = organization.oui
        ));
    }

    Msg::dry_run(format!(
        "Create Helium Organization for NetId {:?}",
        args.net_id,
    ))
}

pub async fn create_roaming_org(args: CreateRoaming) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let (organization, ix) = OrgSolanaOperations::create_org(
            &solana_client,
            args.owner.clone(),
            args.owner.clone(),
            OrgType::Roamer(args.net_id.into()),
        )
        .await?;

        solana_client.send_instruction(ix, &[], None).await?;

        let (_, organization) =
            organization::ensure_exists(&solana_client, OrgIdentifier::Pubkey(organization))
                .await?;

        return Msg::ok(format!(
            "== Roaming Organization Created: {oui} ==\n== Call `org get --oui {oui} to see its details ==",
            oui = organization.oui
        ));
    }

    Msg::dry_run(format!(
        "Create Roaming Organization for NetId {}",
        args.net_id
    ))
}

pub async fn approve_org(args: ApproveOrg) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let ix = OrgSolanaOperations::approve(&solana_client, args.oui).await?;

        solana_client.send_instruction(ix, &[], None).await?;

        return Msg::ok(format!("== Organization Approved: {} ==", args.oui));
    }

    Msg::dry_run(format!("Approve Organization {}", args.oui))
}

pub async fn enable_org(args: EnableOrg) -> Result<Msg> {
    if args.commit {
        let mut client = clients::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        client.enable(args.oui, args.keypair.to_keypair()?).await?;
        return Msg::ok(format!("OUI {} enabled", args.oui));
    }

    Msg::dry_run(format!("enable OUI {}", args.oui))
}

pub async fn disable_org(args: DisableOrg) -> Result<Msg> {
    if args.commit {
        let mut client = clients::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        client.disable(args.oui, args.keypair.to_keypair()?).await?;
        return Msg::ok(format!("OUI {} disabled", args.oui));
    }

    Msg::dry_run(format!("disable OUI {}", args.oui))
}

pub async fn update_owner(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let (_, update_ix) =
            OrgSolanaOperations::update_owner(&solana_client, args.oui, args.pubkey.clone())
                .await?;

        solana_client.send_instruction(update_ix, &[], None).await?;

        return Msg::ok(format!(
            "== Organization Updated: {organization} ==\n== New Owner: {owner} ==",
            organization = args.oui,
            owner = args.pubkey
        ));
    }

    Msg::dry_run(format!(
        "update organization: owner pubkey {}",
        &args.pubkey
    ))
}

pub async fn add_delegate_key(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let ix =
            OrgSolanaOperations::add_delegate_key(&solana_client, args.oui, args.pubkey.clone())
                .await?;

        solana_client.send_instruction(ix, &[], None).await?;

        return Msg::ok(format!(
            "== Organization Updated ==\n== Delegate Added: {delegate} ==",
            delegate = args.pubkey
        ));
    }

    Msg::dry_run(format!(
        "update organization: add delegate key {}",
        &args.pubkey
    ))
}

pub async fn remove_delegate_key(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let ix =
            OrgSolanaOperations::remove_delegate_key(&solana_client, args.oui, args.pubkey).await?;

        solana_client.send_instruction(ix, &[], None).await?;

        return Msg::ok(format!(
            "== Organization Updated ==\n== Call `org get --oui {oui} to see its details ==",
            oui = args.oui
        ));
    }

    Msg::dry_run(format!(
        "update organization: remove delegate key {}",
        &args.pubkey
    ))
}

pub async fn add_devaddr_constraint(args: DevaddrUpdateConstraint) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let ix =
            OrgSolanaOperations::add_devaddr_constraint(&solana_client, args.oui, args.num_blocks)
                .await?;

        solana_client.send_instruction(ix, &[], None).await?;

        return Msg::ok(format!(
            "== Organization Updated ==\n== Call `org get --oui {oui} to see its details ==",
            oui = args.oui
        ));
    }

    Msg::dry_run(format!(
        "update organization: add devaddr constraint {} - num blocks",
        args.num_blocks,
    ))
}

pub async fn remove_devaddr_constraint(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let keypair = args.keypair.to_keypair()?;
        let solana_client = create_solana_client(&args.solana_url, keypair).await?;
        let ix =
            OrgSolanaOperations::remove_devaddr_constraint(&solana_client, args.pubkey).await?;

        solana_client.send_instruction(ix, &[], None).await?;

        return Msg::ok(format!(
            "== Organization Updated ==\n== Call `org get --oui {oui} to see its details ==",
            oui = args.oui
        ));
    }

    Msg::dry_run(format!(
        "update organization: remove devaddr constraint {}",
        &args.pubkey
    ))
}
