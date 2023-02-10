use crate::{client, cmds::PathBufKeypair, region_params::RegionParams, Msg, PrettyJson, Result};
use anyhow::Context;
use helium_proto::Region as ProtoRegion;
use std::{
    fs::{self, File},
    io::Read,
};

use super::PushRegionParams;

pub async fn push_params(args: PushRegionParams) -> Result<Msg> {
    let mut client = client::GatewayClient::new(&args.config_host).await?;
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
        return Msg::dry_run(params.pretty_json()?);
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
            "created region params {}\n{}",
            ProtoRegion::from(args.region),
            params.pretty_json()?
        )),
        Err(err) => Msg::err(format!("region params not created: {err}")),
    }
}
