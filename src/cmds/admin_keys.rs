use crate::{client, Msg, Result};

use super::{AddAdminKey, PathBufKeypair, RemoveAdminKey};

pub async fn add_key(args: AddAdminKey) -> Result<Msg> {
    let mut client = client::AdminClient::new(&args.config_host).await?;

    let response_msg = format!(
        "Pubkey {} with type {} registered",
        &args.pubkey, &args.key_type
    );

    if args.commit {
        _ = client
            .add_key(
                &args.pubkey,
                args.key_type.to_owned().into(),
                &args.keypair.to_keypair()?,
            )
            .await?;
        return Msg::ok(response_msg);
    };
    Msg::dry_run(response_msg)
}

pub async fn remove_key(args: RemoveAdminKey) -> Result<Msg> {
    let mut client = client::AdminClient::new(&args.config_host).await?;

    let response_msg = format!("Pubkey {} de-registered", &args.pubkey);
    if args.commit {
        _ = client
            .remove_key(&args.pubkey, &args.keypair.to_keypair()?)
            .await?;
        return Msg::ok(response_msg);
    };
    Msg::dry_run(response_msg)
}
