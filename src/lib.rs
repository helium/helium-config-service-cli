pub mod hex_field;
pub mod region;
pub mod route;
pub mod server;

use anyhow::Error;
use hex_field::HexField;
use route::Route;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod proto {
    pub use helium_proto::services::config::{
        DevaddrRangeV1, EuiV1, OrgListResV1, OrgV1, RouteListResV1,
    };
}

pub type Result<T = (), E = Error> = anyhow::Result<T, E>;

pub trait PrettyJson {
    fn print_pretty_json(&self) -> Result;
}

impl<S: ?Sized + serde::Serialize> PrettyJson for S {
    fn print_pretty_json(&self) -> Result {
        let pretty = serde_json::to_string_pretty(&self)?;
        println!("{pretty}");
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct OrgList {
    orgs: Vec<Org>,
}

#[derive(Debug, Serialize)]
pub struct Org {
    oui: u64,
    owner: String,
    payer: String,
    nonce: u32,
}

impl Org {
    pub fn new(oui: u64, owner: &str) -> Self {
        Self {
            oui,
            owner: owner.into(),
            payer: owner.into(),
            nonce: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RouteList {
    routes: Vec<Route>,
}

impl RouteList {
    pub fn write_all(&self, out_dir: &Path) -> Result {
        for route in &self.routes {
            route.write(out_dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevaddrRange {
    #[serde(with = "HexField::<8>")]
    start_addr: HexField<8>,
    #[serde(with = "HexField::<8>")]
    end_addr: HexField<8>,
}

impl DevaddrRange {
    pub fn new(start_addr: &str, end_addr: &str) -> Result<Self> {
        Ok(Self {
            start_addr: HexField(u64::from_str_radix(start_addr, 16)?),
            end_addr: HexField(u64::from_str_radix(end_addr, 16)?),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Eui {
    #[serde(with = "HexField::<16>")]
    app_eui: HexField<16>,
    #[serde(with = "HexField::<16>")]
    dev_eui: HexField<16>,
}

impl Eui {
    pub fn new(app_eui: &str, dev_eui: &str) -> Result<Self> {
        Ok(Self {
            app_eui: HexField(u64::from_str_radix(app_eui, 16)?),
            dev_eui: HexField(u64::from_str_radix(dev_eui, 16)?),
        })
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
            owner: String::from_utf8(org.owner).unwrap(),
            payer: String::from_utf8(org.payer).unwrap(),
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
            start_addr: HexField(range.start_addr),
            end_addr: HexField(range.end_addr),
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
            app_eui: HexField(range.app_eui),
            dev_eui: HexField(range.dev_eui),
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
    use crate::{DevaddrRange, Eui, HexField};

    #[test]
    fn deserialize_devaddr() {
        let d = r#"{"start_addr": "11223344", "end_addr": "22334455"}"#;
        let val: DevaddrRange = serde_json::from_str(d).unwrap();
        assert_eq!(
            DevaddrRange {
                start_addr: HexField::<8>(0x11223344),
                end_addr: HexField::<8>(0x22334455)
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
                app_eui: HexField::<16>(0x1122334411223344),
                dev_eui: HexField::<16>(0x2233445522334455)
            },
            val
        );
    }
}
