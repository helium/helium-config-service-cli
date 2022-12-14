/// == Migrating EUIs from Router ==
///
/// Grab the endpoint and token from a Router instance.
/// First get a remote_console
/// ```
/// router remote_console
/// ```
///
/// Then in the erlang repl
/// ```
/// rp(ets:lookup(router_console_api_ets, token)).
/// ```
///
/// It will output something like this
/// ```
/// [{token, {<ENDPOINT>, downlink_endpoint, <TOKEN>}}]
/// ```
///
/// cargo run --bin migrate -- \
///   --console-endpoint <ENDPOINT> \
///   --console-token <TOKEN> \
///   --config-host http://localhost:50051 \
///   --route-id a8f964ce-3a9d-4d72-9bdf-244c9291f2a6 \
///   --keypair ./keypair.bin
///
/// Running without --commit will print how many device EUI's will be sent to the config service.
use anyhow::Result;
use clap::Parser;
use helium_config_service_cli::{client, cmds::PathBufKeypair, Eui};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::runtime::Builder;

fn main() -> Result<()> {
    let args = Args::parse();
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    let mut grpc_client = runtime.block_on(client::RouteClient::new(&args.config_host))?;
    let keypair = args.keypair.to_keypair().expect("valid keypair file");
    let console_devices = ConsoleDevices::new(
        format!("{}/api/router/devices", args.console_endpoint),
        args.console_token,
    )?;

    let euis: Vec<_> = console_devices.map(Eui::from).collect();

    if args.commit {
        println!(
            "Migrating {} devices to route {}",
            euis.len(),
            args.route_id
        );

        let res = runtime.block_on(grpc_client.euis(args.route_id, euis, &keypair))?;
        println!("res: {res:?}");
    } else {
        println!(
            "DRY RUN: Migrating {} devices to route {}",
            euis.len(),
            args.route_id
        );
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    console_endpoint: String,
    #[arg(long)]
    console_token: String,
    #[arg(long)]
    route_id: String,
    #[arg(long)]
    config_host: String,
    #[arg(long, default_value = "./keypair.bin")]
    keypair: PathBuf,
    #[arg(long)]
    commit: bool,
}

#[derive(Debug, Deserialize)]
struct Page {
    data: Vec<Eui>,
    after: Option<String>,
}

#[derive(Debug)]
struct ConsoleDevices {
    devices: <Vec<Eui> as IntoIterator>::IntoIter,
    client: reqwest::blocking::Client,
    after: Option<String>,
    endpoint: String,
    token: String,
    page_num: u8,
}

impl ConsoleDevices {
    fn new(endpoint: String, token: String) -> Result<Self> {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()?
            .json::<Page>()?;
        Ok(Self {
            devices: resp.data.into_iter(),
            client,
            after: resp.after,
            endpoint,
            token,
            page_num: 0,
        })
    }
    fn try_next(&mut self) -> Result<Option<Eui>> {
        // Use the next device from the list if there is one
        if let Some(device) = self.devices.next() {
            return Ok(Some(device));
        }

        // Devices is exhausted, we need to get the next page.
        if let Some(after) = self.after.take() {
            self.page_num += 1;
            println!("fetch page: {}", self.page_num);
            let next_page = self
                .client
                .get(&self.endpoint)
                .bearer_auth(&self.token)
                .query(&[("after", after)])
                .send()?
                .json::<Page>()?;

            self.devices = next_page.data.into_iter();
            self.after = next_page.after;
        }

        Ok(self.devices.next())
    }
}

impl Iterator for ConsoleDevices {
    type Item = Eui;
    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            Ok(Some(device)) => Some(device),
            Ok(None) => None,
            Err(_err) => None,
        }
    }
}
