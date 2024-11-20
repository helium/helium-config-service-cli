use helium_lib::iot_routing_manager::{
    net_id::{self, NetIdIdentifier},
    organization::{self, OrgIdentifier},
};

use super::{
    ApproveOrg, CliSolanaConfig, CreateHelium, CreateNetId, CreateRoaming, DevaddrUpdateConstraint,
    EnableOrg, GetOrg, ListOrgs, OrgUpdateKey, PathBufKeypair,
};

use crate::{
    clients::{self, OrgType},
    helium_netids, Msg, PrettyJson, Result,
};

async fn initialize_clients(
    config_host: &str,
    config_pubkey: &str,
    config_solana: &CliSolanaConfig,
) -> Result<(clients::OrgClient, clients::SolanaClient)> {
    let org_client = clients::OrgClient::new(config_host, config_pubkey).await?;
    let solana_client =
        helium_lib::client::SolanaClient::new(&config_solana.url, config_solana.wallet.clone())?;

    Ok((org_client, solana_client))
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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let (_, ix) = client
            .create_net_id(&solana_client, args.net_id.into())
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

        let (_, net_id) =
            net_id::ensure_exists(&solana_client, NetIdIdentifier::Id(args.net_id.into())).await?;

        return Msg::ok(format!("== NetId Created: {id} ==", id = net_id.id));
    }

    Msg::dry_run(format!("Create NetId {:?}", args.net_id,))
}

pub async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    if args.commit {
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let netid_field = helium_netids::HeliumNetId::from(args.net_id);
        let (organization_key, ix) = client
            .create_org(
                &solana_client,
                args.owner,
                args.owner,
                OrgType::Helium(netid_field),
            )
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let (organization_key, ix) = client
            .create_org(
                &solana_client,
                args.owner,
                args.owner,
                OrgType::Roamer(args.net_id.into()),
            )
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

        return Msg::ok(format!(
            "== Roaming Organization Created: {organization} ==\n== Call `org get --oui {organization} to see its details ==",
            organization = organization_key
        ));
    }

    Msg::dry_run(format!(
        "Create Roaming Organization for NetId {}",
        args.net_id
    ))
}

pub async fn approve_org(args: ApproveOrg) -> Result<Msg> {
    if args.commit {
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client.approve(&solana_client, args.oui).await?;

        solana_client
            .send_instructions(vec![ix], &[], false)
            .await?;

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

pub async fn update_owner(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let (organization_key, update_ix) = client
            .update_owner(&solana_client, args.oui, args.pubkey)
            .await?;

        solana_client
            .send_instructions(vec![update_ix], &[], true)
            .await?;

        return Msg::ok(format!(
            "== Organization Updated: {organization} ==\n== New Owner: {owner} ==",
            organization = organization_key,
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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .add_delegate_key(&solana_client, args.oui, args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .remove_delegate_key(&solana_client, args.oui, args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .add_devaddr_constraint(&solana_client, args.oui, args.num_blocks, None)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

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
        let (mut client, solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .remove_devaddr_constraint(&solana_client, args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;

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
