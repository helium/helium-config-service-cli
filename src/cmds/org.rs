use super::{
    CreateHelium, CreateRoaming, DevaddrSlabAdd, DevaddrUpdateConstraint, EnableOrg, GetOrg,
    ListOrgs, OrgUpdateKey, PathBufKeypair, ENV_NET_ID, ENV_OUI,
};
use crate::{client, subnet::DevaddrConstraint, Msg, PrettyJson, Result};

pub async fn list_orgs(args: ListOrgs) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
    let org = client.list().await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn get_org(args: GetOrg) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
    let org = client.get(args.oui).await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    let delegates = if let Some(ref delegate_keys) = &args.delegate {
        delegate_keys.to_vec()
    } else {
        vec![]
    };
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let org = client
            .create_helium(
                &args.owner,
                &args.payer,
                delegates,
                args.devaddr_count,
                args.net_id,
                &args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(format!(
            "Helium Organization Created: \n{}",
            org.pretty_json()?
        ));
    }
    Msg::dry_run(format!(
        "create Helium organization for NetId {:?} with {} devaddrs",
        args.net_id, args.devaddr_count
    ))
}

pub async fn create_roaming_org(args: CreateRoaming) -> Result<Msg> {
    let delegates = if let Some(ref delegate_keys) = &args.delegate {
        delegate_keys.to_vec()
    } else {
        vec![]
    };
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let created_org = client
            .create_roamer(
                &args.owner,
                &args.payer,
                delegates,
                args.net_id.into(),
                args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(
            [
                "== Roaming Organization Created ==".to_string(),
                created_org.pretty_json()?,
                "== Environment Variables ==".to_string(),
                format!("{ENV_NET_ID}={}", created_org.net_id),
                format!("{ENV_OUI}={}", created_org.org.oui),
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "create Roaming organization for NetId {}",
        args.net_id
    ))
}

pub async fn enable_org(args: EnableOrg) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        client.enable(args.oui, args.keypair.to_keypair()?).await?;
        return Msg::ok(format!("OUI {} enabled", args.oui));
    }
    Msg::dry_run(format!("enable OUI {}", args.oui))
}

pub async fn update_owner(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .update_owner(args.oui, &args.pubkey, args.keypair.to_keypair()?)
            .await?;
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

pub async fn update_payer(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .update_payer(args.oui, &args.pubkey, args.keypair.to_keypair()?)
            .await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: payer pubkey {}",
        &args.pubkey
    ))
}

pub async fn add_delegate_key(args: OrgUpdateKey) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .add_delegate_key(args.oui, &args.pubkey, args.keypair.to_keypair()?)
            .await?;
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
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .remove_delegate_key(args.oui, &args.pubkey, args.keypair.to_keypair()?)
            .await?;
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
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .add_devaddr_slab(args.oui, args.devaddr_count, args.keypair.to_keypair()?)
            .await?;
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
        args.devaddr_count
    ))
}

pub async fn add_devaddr_constraint(args: DevaddrUpdateConstraint) -> Result<Msg> {
    let constraint = DevaddrConstraint::new(args.start_addr, args.end_addr)?;
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .add_devaddr_constraint(args.oui, constraint, args.keypair.to_keypair()?)
            .await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: add devaddr constraint {} - {}",
        constraint.start_addr, constraint.end_addr
    ))
}

pub async fn remove_devaddr_constraint(args: DevaddrUpdateConstraint) -> Result<Msg> {
    let constraint = DevaddrConstraint::new(args.start_addr, args.end_addr)?;
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host, &args.config_pubkey).await?;
        let updated_org = client
            .remove_devaddr_constraint(args.oui, constraint, args.keypair.to_keypair()?)
            .await?;
        return Msg::ok(
            [
                "== Organization Updated ==".to_string(),
                updated_org.pretty_json()?,
            ]
            .join("\n"),
        );
    }
    Msg::dry_run(format!(
        "update organization: remove devaddr constraint {} - {}",
        constraint.start_addr, constraint.end_addr
    ))
}
