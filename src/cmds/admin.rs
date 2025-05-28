use crate::{clients, cmds::PathBufKeypair, region_params::RegionParams, Msg, Result};
use anyhow::Context;
use helium_proto::Region as ProtoRegion;
use std::{
    fs::{self, File},
    io::Read,
};

use super::{AdminAddKey, AdminLoadRegionParams, AdminRemoveKey};

pub async fn add_key(args: AdminAddKey) -> Result<Msg> {
    if args.commit {
        let mut client = clients::AdminClient::new(&args.config_host, &args.config_pubkey).await?;
        client
            .add_key(&args.pubkey, args.key_type, &args.keypair.to_keypair()?)
            .await?;

        return Msg::ok(format!("Added {} as {} key", args.pubkey, args.key_type));
    }
    Msg::dry_run(format!("Added {} as {} key", args.pubkey, args.key_type))
}

pub async fn remove_key(args: AdminRemoveKey) -> Result<Msg> {
    if args.commit {
        let mut client = clients::AdminClient::new(&args.config_host, &args.config_pubkey).await?;
        client
            .remove_key(&args.pubkey, &args.keypair.to_keypair()?)
            .await?;
        return Msg::ok(format!("Removed key {}", args.pubkey));
    }
    Msg::dry_run(format!("Removed key {}", args.pubkey))
}

pub async fn load_region(args: AdminLoadRegionParams) -> Result<Msg> {
    let mut client = clients::AdminClient::new(&args.config_host, &args.config_pubkey).await?;
    let params = RegionParams::from_file(&args.params_file)?;

    let index_bytes = if let Some(index_path) = &args.index_file {
        let mut index_file = File::open(index_path).context("reading region h3 indices file")?;
        let metadata = fs::metadata(index_path).context("reading index file metadata")?;
        let mut byte_buf = vec![0; metadata.len() as usize];
        index_file
            .read(&mut byte_buf)
            .context("reading index buffer")?;

        byte_buf
    } else {
        vec![]
    };

    if !args.commit {
        return Msg::dry_run(format!(
            "params loaded for region {}",
            ProtoRegion::from(args.region)
        ));
    }

    match client
        .load_region(
            args.region.clone(),
            params.clone(),
            index_bytes,
            &args.keypair.to_keypair()?,
        )
        .await
    {
        Ok(_) => Msg::ok(format!(
            "params loaded for region {}",
            ProtoRegion::from(args.region)
        )),
        Err(err) => Msg::err(format!("region params not created: {err}")),
    }
}
