use super::{
    CliSolanaConfig, CreateHelium, CreateRoaming, DevaddrSlabAdd, DevaddrUpdateConstraint,
    EnableOrg, GetOrg, ListOrgs, OrgUpdateKey, PathBufKeypair, ENV_NET_ID, ENV_OUI,
};

use crate::{
    clients::{self, SolanaClientOpts},
    helium_netids, lora_field, Msg, PrettyJson, Result,
};

async fn initialize_clients(
    config_host: &str,
    config_pubkey: &str,
    config_solana: &CliSolanaConfig,
) -> Result<(clients::OrgClient, clients::SolanaClient)> {
    let solana_opts = SolanaClientOpts {
        wallet: config_solana.solana_wallet.clone(),
        url: config_solana.solana_url.clone(),
    };

    let org_client = clients::OrgClient::new(config_host, config_pubkey).await?;
    let solana_client = clients::SolanaClient::new(solana_opts.clone()).await?;
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

pub async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    let delegates = args
        .delegate
        .as_ref()
        .map_or_else(Vec::new, |keys| keys.to_vec());

    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let netid_field = helium_netids::HeliumNetId::from(args.net_id);
        let (oui, ixs) = client
            .create_helium(
                &solana_client,
                args.owner,
                delegates,
                args.devaddr_num_blocks,
                netid_field,
            )
            .await?;

        solana_client.send_instructions(ixs, &[], true).await?;

        return Msg::ok(format!(
            "== Helium Organization Created, oui: {oui} ==\n== Call `org get --oui {oui} to see its details` ==",
            oui = oui
        ));
    }

    Msg::dry_run(format!(
        "create Helium organization for NetId {:?} with {} devaddrs",
        args.net_id,
        args.devaddr_num_blocks * 8
    ))
}

pub async fn create_roaming_org(args: CreateRoaming) -> Result<Msg> {
    let delegates = args
        .delegate
        .as_ref()
        .map_or_else(Vec::new, |keys| keys.to_vec());

    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let (oui, ixs) = client
            .create_roamer(&solana_client, args.owner, delegates, args.net_id.into())
            .await?;

        solana_client
            .send_instructions(ixs, &Vec::new(), true)
            .await?;

        return Msg::ok(format!(
            "== Roaming Organization Created, oui: {oui} ==\n== Call `org get --oui {oui} to see its details ==",
            oui = oui
        ));
    }

    Msg::dry_run(format!(
        "create Roaming organization for NetId {}",
        args.net_id
    ))
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
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .update_owner(&solana_client, args.oui, args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: owner pubkey {}",
        &args.pubkey
    ))
}

pub async fn add_delegate_key(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .add_delegate_key(&solana_client, args.oui, &args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: add delegate key {}",
        &args.pubkey
    ))
}

pub async fn remove_delegate_key(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .remove_delegate_key(&solana_client, args.oui, &args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: remove delegate key {}",
        &args.pubkey
    ))
}

pub async fn add_devaddr_slab(args: DevaddrSlabAdd) -> Result<Msg> {
    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .add_devaddr_constraint(&solana_client, args.oui, args.devaddr_num_blocks)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: add {} new devaddrs",
        args.devaddr_num_blocks
    ))
}

pub async fn add_devaddr_constraint(args: DevaddrUpdateConstraint) -> Result<Msg> {
    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .add_devaddr_constraint(&solana_client, args.oui, args.num_blocks)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: add devaddr constraint {} - num blocks",
        args.num_blocks,
    ))
}

pub async fn remove_devaddr_constraint(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let (mut client, mut solana_client) =
            initialize_clients(&args.config_host, &args.config_pubkey, &args.solana).await?;

        let ix = client
            .remove_devaddr_constraint(&solana_client, args.oui, args.pubkey)
            .await?;

        solana_client.send_instructions(vec![ix], &[], true).await?;
        let updated_org = client.get(args.oui).await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: remove devaddr constraint {}",
        &args.pubkey
    ))
}
