#![recursion_limit = "512"]
/// == Migrating Packet Purchaser to Config Service ==
///
/// Make a JSON file that maps string NetIDs to OUIs
/// ```
/// {
///     "C00053": 1234
/// }
/// ```
///
/// Read the .env file of packet-purchaser or run the following in a remote_console.
///
/// ```
/// rr(pp_console_ws_manager).
/// State = recon:get_state(pp_console_ws_manager).
/// io:format("Endpoint: ~p~nSecret: ~p~n", [State#state.http_endpoint, State#state.secret]).
/// ```
///
/// Drop the protocol from the endpoint.
///
/// Usage:
/// ```
/// cargo run --bin roaming -- \
///     --endpoint <ENDPOINT> \
///     --secret <SECRET> \
///     --net-id-oui-file mappings.json
/// ```
use anyhow::Result;
use clap::Parser;
use helium_config_service_cli::{
    hex_field::HexNetID,
    route::Route,
    server::{FlowType, Protocol, Server},
    DevaddrRange,
};
use reqwest::Url;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::{info, warn};
use websocket::{native_tls::TlsConnector, ws::dataframe::DataFrame, ClientBuilder, Message};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // Read in NetID -> OUI file.
    let mappings: HashMap<String, u64> = {
        let data = fs::read_to_string(args.net_id_oui_file).expect("file exists");
        serde_json::from_str(&data).expect("valid mapping file")
    };

    // Connect to the roaming consoles websocket.
    let url = get_url_with_token(&args.endpoint, &args.secret).await?;
    info!("got token");
    let connector = TlsConnector::new()?;
    let mut client = ClientBuilder::new(&url)
        .unwrap()
        .connect(Some(connector))
        .unwrap();
    info!("ws connected");

    // Join the org channel so we can request all the configurations.
    info!("joining org channel");
    let phx_message = json!([0, 0, "organization:all", "phx_join", {}]);
    let serialized = serde_json::to_string(&phx_message)?;
    let message = Message::text(serialized);
    client.send_message(&message).unwrap();
    let _channel_join_success = client.recv_message()?;

    // Receive the configuration, throwing away all the extra pheonix message cruft.
    // We're left with a serde_json::Value that can be deserialized into a ConfigList.
    let config_payload = {
        info!("requesting configuration");
        let config_msg = json!([0, 0, "organization:all", "packet_purchaser:get_config", {}]);
        let serialized = serde_json::to_string(&config_msg)?;
        let message = Message::text(serialized);
        client.send_message(&message).unwrap();
        let config_list_msg = client.recv_message()?.take_payload();
        // Messages come back as arrays with 5 elements.
        // Parse into Vec<Value> so we can throw away the first 4 elements.
        // [jref, ref, topic, event, payload]
        let parsed_msg: Vec<serde_json::Value> =
            serde_json::from_str(std::str::from_utf8(&config_list_msg)?).expect("parsing");
        parsed_msg.get(4).expect("Configuration payload").clone()
    };

    // Map Config's into Route's and write them to the ./roaming folder.
    info!("processing routes");
    let configs: ConfigList = serde_json::from_value(config_payload)?;
    for org in configs.org_config_list {
        for (idx, config) in org.configs.into_iter().enumerate() {
            let oui = mappings
                .get(&org.net_id.to_string())
                .unwrap_or_else(|| {
                    warn!("unhandled NetID {:?}", org.net_id.to_string());
                    &1337
                })
                .to_owned();
            let route = to_route(config, org.net_id, oui);
            let filename = format!("./roaming/{}-route-{}.json", org.name, idx);
            route.write(Path::new(&filename))?;
            info!("wrote {filename}");
        }
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    /// Without the protocol, provide the baseurl for the roaming console.
    /// No path.
    #[arg(long)]
    endpoint: String,

    #[arg(long)]
    secret: String,

    #[arg(long)]
    net_id_oui_file: PathBuf,
}

/// Phoenix requires outer messages to be objects.
#[derive(Debug, Deserialize)]
struct ConfigList {
    org_config_list: Vec<OrgConfigs>,
}

/// Configurations are grouped by the Organization
#[derive(Debug, Deserialize)]
struct OrgConfigs {
    name: String,
    net_id: HexNetID,
    configs: Vec<Config>,
}

/// Config is the old style Route
#[derive(Debug, Deserialize)]
struct Config {
    devaddrs: Vec<DevaddrRange>,
    address: Option<String>,
    port: Option<u32>,
    http_dedupe_timeout: Option<u32>,
    http_endpoint: Option<String>,
    joins: Vec<helium_config_service_cli::Eui>,
    multi_buy: Option<u32>,
    protocol: String,
    // Unused fields:
    // active: bool,
    // http_auth_header: Option<String>,
    // http_flow_type: Option<String>,
    // protocol_version: String,
}

async fn get_url_with_token(endpoint: &str, secret: &str) -> Result<String> {
    #[derive(Debug, Deserialize)]
    struct Token {
        jwt: String,
    }

    let http_client = reqwest::Client::new();

    let token = http_client
        .post(format!("https://{endpoint}/api/packet_purchaser/sessions"))
        .json(&HashMap::from([("secret", secret)]))
        .send()
        .await?
        .json::<Token>()
        .await?;

    let url = format!(
        "wss://{endpoint}/socket/packet_purchaser/websocket?token={}&vsn=2.0.0",
        token.jwt
    );
    Ok(url)
}

fn to_route(config: Config, net_id: HexNetID, oui: u64) -> Route {
    let server = match config.protocol.as_str() {
        "udp" => Server::new(
            config.address.unwrap(),
            config.port.unwrap(),
            Protocol::default_gwmp(),
        ),
        "http" => {
            let url = config
                .http_endpoint
                .unwrap()
                .parse::<Url>()
                .expect("valid url");

            Server::new(
                url.as_str().to_string(),
                url.port().unwrap_or(80) as u32,
                Protocol::make_http(
                    FlowType::Async,
                    config.http_dedupe_timeout.unwrap_or(250),
                    url.path().into(),
                ),
            )
        }
        _ => panic!("bad protocol"),
    };
    Route {
        // No ID, that will be added when the route is pushed to the config service.
        id: "".into(),
        net_id,
        devaddr_ranges: config.devaddrs,
        euis: config.joins,
        oui,
        server,
        max_copies: config.multi_buy.unwrap_or(0),
        nonce: 0,
    }
}

#[cfg(test)]
mod tests {

    use crate::{to_route, Config, OrgConfigs};
    use helium_config_service_cli::{hex_field, Eui};
    use std::collections::HashMap;

    #[test]
    fn config_to_route() {
        let x = Config {
            devaddrs: vec![],
            address: None,
            port: None,
            http_dedupe_timeout: Some(20),
            http_endpoint: Some("http://example.com".to_string()),
            joins: vec![
                Eui::new(hex_field::eui(0), hex_field::eui(2615410877735726668)).unwrap(),
                Eui::new(hex_field::eui(0), hex_field::eui(10307799872453973643)).unwrap(),
            ],
            multi_buy: Some(0),
            protocol: "http".to_string(),
        };
        let route = to_route(x, hex_field::net_id(1), 2);
        println!("{route:#?}");
    }

    #[test]
    fn config_to_route_2() {
        let x = Config {
            devaddrs: vec![],
            address: None,
            port: None,
            http_dedupe_timeout: None,
            http_endpoint: Some("http://example.com".to_string()),
            joins: vec![],
            multi_buy: Some(5),
            protocol: "http".to_string(),
        };
        let route = to_route(x, hex_field::net_id(1), 2);
        println!("{route:#?}");
    }

    #[test]
    fn org_parse() {
        let val = serde_json::json!({"name": "michael", "net_id": 12582992, "configs": []});
        let org: OrgConfigs = serde_json::from_value(val).unwrap();
        println!("{org:#?}");
    }

    #[test]
    fn lookup_mapping() {
        let mappings: HashMap<String, u32> = HashMap::from([("60004A".to_string(), 1337)]);
        let net_id = hex_field::net_id(6291530);
        println!("{:?} => {:?}", net_id, mappings.get(&net_id.to_string()))
    }
}
