use helium_config_service_cli::{
    cmds::{self, *},
    Result,
};
use tracing::info;

#[tokio::test]
async fn get_orgs() -> Result {
    tracing_subscriber::fmt::init();

    let out = cmds::org::get_orgs(GetOrgs {
        // config_host: "http://127.0.0.1:50051".to_string(),
        config_host: "https://alb.iot.mainnet.helium.io:6080".to_string(),
    })
    .await?;
    info!("{out}");

    Ok(())
}
