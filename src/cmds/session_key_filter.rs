use super::{AddFilter, GetFilters, ListFilters, PathBufKeypair, RemoveFilter};
use crate::{client, Msg, PrettyJson, Result, SessionKeyFilter};

pub async fn list_filters(args: ListFilters) -> Result<Msg> {
    let mut client = client::SkfClient::new(&args.config_host).await?;
    let filters = client
        .list_filters(args.oui, &args.keypair.to_keypair()?)
        .await?;

    Msg::ok(filters.pretty_json()?)
}

pub async fn get_filters(args: GetFilters) -> Result<Msg> {
    let mut client = client::SkfClient::new(&args.config_host).await?;
    let filters = client
        .get_filters(args.oui, args.devaddr, &args.keypair.to_keypair()?)
        .await?;

    Msg::ok(filters.pretty_json()?)
}

pub async fn add_filter(args: AddFilter) -> Result<Msg> {
    let mut client = client::SkfClient::new(&args.config_host).await?;
    let filter = SessionKeyFilter::new(args.oui, args.devaddr, args.session_key);

    if !args.commit {
        return Msg::dry_run(format!("added {filter:?}"));
    }

    client
        .add_filters(vec![filter.clone()], &args.keypair.to_keypair()?)
        .await?;

    Msg::ok(format!("added {filter:?}"))
}

pub async fn remove_filter(args: RemoveFilter) -> Result<Msg> {
    let mut client = client::SkfClient::new(&args.config_host).await?;
    let filter = SessionKeyFilter::new(args.oui, args.devaddr, args.session_key);

    if !args.commit {
        return Msg::dry_run(format!("removed {filter:?}"));
    }

    client
        .remove_filters(vec![filter.clone()], &args.keypair.to_keypair()?)
        .await?;

    Msg::ok(format!("removed {filter:?}"))
}
