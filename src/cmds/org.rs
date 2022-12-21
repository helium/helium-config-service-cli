use super::{CreateHelium, CreateRoaming, GetOrg, GetOrgs, PathBufKeypair, ENV_NET_ID, ENV_OUI};
use crate::{client, Msg, PrettyJson, Result};
pub async fn get_orgs(args: GetOrgs) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host).await?;
    let org = client.list().await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn get_org(args: GetOrg) -> Result<Msg> {
    let mut client = client::OrgClient::new(&args.config_host).await?;
    let org = client.get(args.oui).await?;

    Msg::ok(org.pretty_json()?)
}

pub async fn create_helium_org(args: CreateHelium) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host).await?;
        let org = client
            .create_helium(
                &args.owner,
                &args.payer,
                args.devaddr_count,
                args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(format!(
            "Helium Organization Created: \n{}",
            org.pretty_json()?
        ));
    }
    Msg::ok("pass `--commit` to create Helium organization".to_string())
}

pub async fn create_roaming_org(args: CreateRoaming) -> Result<Msg> {
    if args.commit {
        let mut client = client::OrgClient::new(&args.config_host).await?;
        let created_org = client
            .create_roamer(
                &args.owner,
                &args.payer,
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
    Msg::ok("pass `--commit` to create Roaming organization".to_string())
}
