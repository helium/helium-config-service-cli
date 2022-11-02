pub mod hex_field;

use helium_proto::services::config::{
    server_v1::Protocol, DevaddrRangeV1, EuiV1, OrgListResV1, OrgV1, RouteListResV1, RouteV1,
    ServerV1,
};
use hex_field::HexField;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait PrettyJson {
    fn print_pretty_json(&self) -> Result<()>;
}

impl<S: ?Sized + serde::Serialize> PrettyJson for S {
    fn print_pretty_json(&self) -> Result<()> {
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
    pub fn new(oui: u64) -> Self {
        Self {
            oui,
            owner: "owner".into(),
            payer: "payer".into(),
            nonce: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RouteList {
    routes: Vec<Route>,
}

impl RouteList {
    pub fn write_all(&self, out_dir: &Path) -> Result<()> {
        for route in &self.routes {
            route.write(out_dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Route {
    id: String,
    #[serde(with = "HexField::<6>")]
    net_id: HexField<6>,
    pub devaddr_ranges: Vec<DevaddrRange>,
    pub euis: Vec<Eui>,
    oui: u64,
    server: Option<Server>,
    max_copies: u32,
    nonce: u32,
}

impl Route {
    pub fn new(net_id: HexField<6>, oui: u64, max_copies: u32) -> Self {
        Self {
            id: "".into(),
            net_id,
            devaddr_ranges: vec![],
            euis: vec![],
            oui,
            server: None,
            max_copies,
            nonce: 1,
        }
    }
    pub fn from_file(dir: &Path, id: String) -> Result<Self> {
        let filename = dir.join(id).with_extension("json");
        let data = fs::read_to_string(filename).expect("Could not read file");
        let listing: Self = serde_json::from_str(&data)?;
        Ok(listing)
    }

    pub fn filename(&self) -> String {
        format!("{}.json", self.id.clone())
    }

    pub fn write(&self, out_dir: &Path) -> Result<()> {
        let data = serde_json::to_string_pretty(&self)?;
        let filename = out_dir.join(self.filename());
        fs::write(filename, data).expect("unable to write file");
        Ok(())
    }

    pub fn remove(&self, out_dir: &Path) -> Result<()> {
        let filename = out_dir.join(self.filename());
        fs::remove_file(filename)?;
        Ok(())
    }

    pub fn inc_nonce(self) -> Self {
        Self {
            nonce: self.nonce + 1,
            ..self
        }
    }

    pub fn add_eui(&mut self, eui: Eui) {
        self.euis.push(eui);
    }

    pub fn add_devaddr(&mut self, range: DevaddrRange) {
        self.devaddr_ranges.push(range);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Server {
    host: String,
    port: u32,
    protocol: Option<Protocol>,
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

impl From<OrgListResV1> for OrgList {
    fn from(org_list: OrgListResV1) -> Self {
        Self {
            orgs: org_list.orgs.into_iter().map(|o| o.into()).collect(),
        }
    }
}

impl From<OrgV1> for Org {
    fn from(org: OrgV1) -> Self {
        Self {
            oui: org.oui,
            owner: String::from_utf8(org.owner).unwrap(),
            payer: String::from_utf8(org.payer).unwrap(),
            nonce: org.nonce,
        }
    }
}

impl From<Org> for OrgV1 {
    fn from(org: Org) -> Self {
        Self {
            oui: org.oui,
            owner: org.owner.into(),
            payer: org.payer.into(),
            nonce: org.nonce,
        }
    }
}

impl From<RouteListResV1> for RouteList {
    fn from(route_list: RouteListResV1) -> Self {
        Self {
            routes: route_list.routes.into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<RouteV1> for Route {
    fn from(route: RouteV1) -> Self {
        Self {
            id: String::from_utf8(route.id).unwrap(),
            net_id: HexField::<6>(route.net_id),
            devaddr_ranges: route.devaddr_ranges.into_iter().map(|r| r.into()).collect(),
            euis: route.euis.into_iter().map(|e| e.into()).collect(),
            oui: route.oui,
            server: route.server.map(|s| s.into()),
            max_copies: route.max_copies,
            nonce: route.nonce,
        }
    }
}

impl From<Route> for RouteV1 {
    fn from(route: Route) -> Self {
        Self {
            id: route.id.into(),
            net_id: route.net_id.into(),
            devaddr_ranges: route.devaddr_ranges.into_iter().map(|r| r.into()).collect(),
            euis: route.euis.into_iter().map(|e| e.into()).collect(),
            oui: route.oui,
            server: route.server.map(|s| s.into()),
            max_copies: route.max_copies,
            nonce: route.nonce,
        }
    }
}

impl From<Server> for ServerV1 {
    fn from(server: Server) -> Self {
        Self {
            host: server.host.into(),
            port: server.port,
            protocol: server.protocol,
        }
    }
}

impl From<ServerV1> for Server {
    fn from(server: ServerV1) -> Self {
        Self {
            host: String::from_utf8(server.host).unwrap(),
            port: server.port,
            protocol: server.protocol,
        }
    }
}

impl From<DevaddrRangeV1> for DevaddrRange {
    fn from(range: DevaddrRangeV1) -> Self {
        Self {
            start_addr: HexField(range.start_addr),
            end_addr: HexField(range.end_addr),
        }
    }
}

impl From<DevaddrRange> for DevaddrRangeV1 {
    fn from(range: DevaddrRange) -> Self {
        Self {
            start_addr: range.start_addr.0,
            end_addr: range.end_addr.0,
        }
    }
}

impl From<EuiV1> for Eui {
    fn from(range: EuiV1) -> Self {
        Self {
            app_eui: HexField(range.app_eui),
            dev_eui: HexField(range.dev_eui),
        }
    }
}

impl From<Eui> for EuiV1 {
    fn from(range: Eui) -> Self {
        Self {
            app_eui: range.app_eui.0,
            dev_eui: range.dev_eui.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use helium_proto::services::config::{DevaddrRangeV1, EuiV1, RouteV1};

    use crate::{DevaddrRange, Eui, HexField, Route};

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

    #[test]
    fn route_to_route_v1_conversion() {
        let route = Route {
            id: "route_id".into(),
            net_id: HexField(1),
            devaddr_ranges: vec![DevaddrRange {
                start_addr: HexField(287454020),
                end_addr: HexField(2005440768),
            }],
            euis: vec![Eui {
                app_eui: HexField(12302652060662178304),
                dev_eui: HexField(12302652060662178304),
            }],
            oui: 66,
            server: None,
            max_copies: 999,
            nonce: 1337,
        };
        let v1 = RouteV1 {
            id: vec![114, 111, 117, 116, 101, 95, 105, 100],
            net_id: 1,
            devaddr_ranges: vec![DevaddrRangeV1 {
                start_addr: 287454020,
                end_addr: 2005440768,
            }],
            euis: vec![EuiV1 {
                app_eui: 12302652060662178304,
                dev_eui: 12302652060662178304,
            }],
            oui: 66,
            server: None,
            max_copies: 999,
            nonce: 1337,
        };
        assert_eq!(route, Route::from(v1.clone()));
        assert_eq!(v1, RouteV1::from(route));
    }
}
