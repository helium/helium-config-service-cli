pub mod client;
pub mod cmds;
pub mod hex_field;
pub mod region;
pub mod route;
pub mod server;

use anyhow::{anyhow, Context, Error};
use helium_crypto::PublicKey;
use hex_field::HexNetID;
use route::Route;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub mod proto {
    pub use helium_proto::services::config::{
        DevaddrRangeV1, EuiV1, OrgListResV1, OrgResV1, OrgV1, RouteListResV1,
    };
}

pub type Result<T = (), E = Error> = anyhow::Result<T, E>;

pub trait PrettyJson {
    fn print_pretty_json(&self) -> Result;
    fn pretty_json(&self) -> Result<String>;
}

impl<S: ?Sized + serde::Serialize> PrettyJson for S {
    fn print_pretty_json(&self) -> Result {
        println!("{}", self.pretty_json()?);
        Ok(())
    }

    fn pretty_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).map_err(|e| e.into())
    }
}

#[derive(Debug, Serialize)]
pub struct OrgResponse {
    pub org: Org,
    pub net_id: HexNetID,
    pub devaddr_ranges: Vec<DevaddrRange>,
}

impl From<proto::OrgResV1> for OrgResponse {
    fn from(res: proto::OrgResV1) -> Self {
        Self {
            org: res.org.expect("no org returned during creation").into(),
            net_id: hex_field::net_id(res.net_id),
            devaddr_ranges: res.devaddr_ranges.into_iter().map(|d| d.into()).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrgList {
    orgs: Vec<Org>,
}

#[derive(Debug, Serialize)]
pub struct Org {
    pub oui: u64,
    pub owner: PublicKey,
    pub payer: PublicKey,
    pub nonce: u32,
}

#[derive(Debug, Serialize)]
pub struct RouteList {
    routes: Vec<Route>,
}

impl RouteList {
    pub fn write_all(&self, out_dir: &Path) -> Result {
        fs::create_dir_all(out_dir).context("route list creating parent directory")?;
        for route in &self.routes {
            route.write(out_dir)?;
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.routes.len()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevaddrRange {
    start_addr: hex_field::HexDevAddr,
    end_addr: hex_field::HexDevAddr,
}

impl DevaddrRange {
    pub fn new(start_addr: hex_field::HexDevAddr, end_addr: hex_field::HexDevAddr) -> Result<Self> {
        if end_addr < start_addr {
            return Err(anyhow!("start_addr cannot be greater than end_addr"));
        }

        Ok(Self {
            start_addr,
            end_addr,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eui {
    app_eui: hex_field::HexEui,
    dev_eui: hex_field::HexEui,
}

impl Eui {
    pub fn new(app_eui: hex_field::HexEui, dev_eui: hex_field::HexEui) -> Result<Self> {
        Ok(Self { app_eui, dev_eui })
    }
}

impl From<proto::OrgListResV1> for OrgList {
    fn from(org_list: proto::OrgListResV1) -> Self {
        Self {
            orgs: org_list.orgs.into_iter().map(|o| o.into()).collect(),
        }
    }
}

impl From<proto::OrgV1> for Org {
    fn from(org: proto::OrgV1) -> Self {
        Self {
            oui: org.oui,
            owner: PublicKey::try_from(org.owner).unwrap(),
            payer: PublicKey::try_from(org.payer).unwrap(),
            nonce: org.nonce,
        }
    }
}

impl From<Org> for proto::OrgV1 {
    fn from(org: Org) -> Self {
        Self {
            oui: org.oui,
            owner: org.owner.into(),
            payer: org.payer.into(),
            nonce: org.nonce,
        }
    }
}

impl From<proto::RouteListResV1> for RouteList {
    fn from(route_list: proto::RouteListResV1) -> Self {
        Self {
            routes: route_list.routes.into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<proto::DevaddrRangeV1> for DevaddrRange {
    fn from(range: proto::DevaddrRangeV1) -> Self {
        Self {
            start_addr: range.start_addr.into(),
            end_addr: range.end_addr.into(),
        }
    }
}

impl From<DevaddrRange> for proto::DevaddrRangeV1 {
    fn from(range: DevaddrRange) -> Self {
        Self {
            start_addr: range.start_addr.0,
            end_addr: range.end_addr.0,
        }
    }
}

impl From<proto::EuiV1> for Eui {
    fn from(range: proto::EuiV1) -> Self {
        Self {
            app_eui: range.app_eui.into(),
            dev_eui: range.dev_eui.into(),
        }
    }
}

impl From<Eui> for proto::EuiV1 {
    fn from(range: Eui) -> Self {
        Self {
            app_eui: range.app_eui.0,
            dev_eui: range.dev_eui.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hex_field, DevaddrRange, Eui};

    #[test]
    fn deserialize_devaddr() {
        let d = r#"{"start_addr": "11223344", "end_addr": "22334455"}"#;
        let val: DevaddrRange = serde_json::from_str(d).unwrap();
        assert_eq!(
            DevaddrRange {
                start_addr: hex_field::devaddr(0x11223344),
                end_addr: hex_field::devaddr(0x22334455)
            },
            val
        );
    }

    #[test]
    fn deserialize_eui() {
        let d = r#"{"app_eui": "1122334411223344", "dev_eui": "2233445522334455"}"#;
        let val: Eui = serde_json::from_str(d).unwrap();
        assert_eq!(
            Eui {
                app_eui: hex_field::eui(0x1122334411223344),
                dev_eui: hex_field::eui(0x2233445522334455)
            },
            val
        );
    }
}
