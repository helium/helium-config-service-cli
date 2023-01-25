use helium_config_service_cli::{client, Result};
use helium_proto::services::iot_config::route_client::RouteClient;
use rand::rngs::OsRng;
use tracing::{info, warn};

#[tokio::test]
async fn connect() -> Result {
    tracing_subscriber::fmt::init();

    let keypair = helium_crypto::Keypair::generate(
        helium_crypto::KeyTag {
            network: helium_crypto::Network::MainNet,
            key_type: helium_crypto::KeyType::Ed25519,
        },
        &mut OsRng,
    );

    let mut client =
        RouteClient::connect("https://alb.iot.mainnet.helium.io:6080".to_owned()).await?;

    let request = client::mk_route_stream_request(&keypair);
    let mut stream = client.stream(request).await?.into_inner();

    info!("listening to messages");
    loop {
        let message = stream.message().await;
        match message {
            Ok(ok) => info!("{ok:#?}"),
            Err(err) => {
                warn!("{err:#?}");
                break;
            }
        };
    }
    info!("we got an erro and we're done");

    Ok(())
}
