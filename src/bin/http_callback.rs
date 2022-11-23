use std::{collections::BTreeMap, sync::Arc, time::Duration};

use helium_config_service_cli::Result;
use http_callback::{roaming_downlinks_server, Downlink, Register};

use tokio::sync::broadcast::Sender;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status};

mod http_callback {
    tonic::include_proto!("http_downlink_handler");
}

#[derive(Debug, Clone)]
pub struct HttpCallback {
    downlink_channel: Arc<tokio::sync::broadcast::Sender<Downlink>>,
}

#[tonic::async_trait]
impl http_callback::roaming_downlinks_server::RoamingDownlinks for HttpCallback {
    type SubscribeStream = ReceiverStream<Result<Downlink, Status>>;

    async fn subscribe(
        &self,
        _request: Request<Register>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let mut downlinks = self.downlink_channel.subscribe();

        tokio::spawn(async move {
            /*
            It would be really nice to cleanup processes as receivers are shut
            down by the GRPC stream handlers. Given the number of connections this little service
            is meant to be handling, that's not an extra complication I think we need to introduce
            at this time.
            */
            while let Ok(downlink) = downlinks.recv().await {
                println!("==> got a thing");
                if let Err(_) = tx.send(Ok(downlink)).await {
                    break;
                }
            }

            println!("Disconnected: {:?}", _request.into_inner());
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result {
    let addr = "0.0.0.0:50051".parse()?;

    let (tx, _rx) = tokio::sync::broadcast::channel(128);
    let sender = Arc::new(tx);
    let service = HttpCallback {
        downlink_channel: sender.clone(),
    };

    let _ = send_fake_downlinks(Duration::from_millis(1000), sender).await?;

    tonic::transport::Server::builder()
        .add_service(roaming_downlinks_server::RoamingDownlinksServer::new(
            service,
        ))
        .serve(addr)
        .await?;

    Ok(())
}

async fn send_fake_downlinks(throttle: Duration, sender: Arc<Sender<Downlink>>) -> Result {
    let mut curr = 1;
    let repeat_simple_downlink = std::iter::repeat_with(move || {
        let tmp = curr;
        curr += 1;
        let a = prost_types::Struct {
            fields: BTreeMap::from([(
                "count".to_string(),
                prost_types::Value {
                    kind: Some(prost_types::value::Kind::NumberValue(tmp.into())),
                },
            )]),
        };
        Downlink { data: Some(a) }
    });

    let mut stream = Box::pin(tokio_stream::iter(repeat_simple_downlink).throttle(throttle));
    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match sender.send(item) {
                Ok(_) => println!("sent on tick success"),
                Err(_) => println!("sent on tock error"),
            }
        }
    });
    Ok(())
}
