use helium_config_service_cli::Result;
use http_callback::{roaming_downlinks_client::RoamingDownlinksClient, Register};

mod http_callback {
    tonic::include_proto!("http_downlink_handler");
}

#[tokio::main]
async fn main() -> Result {
    let mut client = RoamingDownlinksClient::connect("http://127.0.0.1:50051").await?;

    let request = Register {
        timestamp: 0,
        gateway: vec![],
        signature: vec![],
    };
    let mut stream = client.subscribe(request).await?.into_inner();

    while let Ok(item) = stream.message().await {
        println!("<== received {item:?}");
    }

    Ok(())
}
