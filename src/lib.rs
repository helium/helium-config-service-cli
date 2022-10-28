use helium_proto::services::config::{
    server_v1::Protocol, DevaddrRangeV1, EuiV1, OrgListResV1, OrgV1, RouteListResV1, RouteV1,
    ServerV1,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fs, num::ParseIntError, path::PathBuf, str::FromStr};

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

#[derive(Debug, PartialEq)]
pub struct HexField<const WIDTH: usize>(pub u64);

impl<const WIDTH: usize> HexField<WIDTH> {
    pub fn into_inner(&self) -> u64 {
        self.0
    }
}

impl<const WIDTH: usize> Serialize for HexField<WIDTH> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // pad with 0s to the left up to WIDTH
        serializer.serialize_str(&format!("{:0>width$X}", self.0, width = WIDTH))
    }
}

impl<'de, const WIDTH: usize> Deserialize<'de> for HexField<WIDTH> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;
        HexField::<WIDTH>::from_str(&buf).map_err(serde::de::Error::custom)
    }
}

impl<const WIDTH: usize> FromStr for HexField<WIDTH> {
    type Err = ParseIntError;
    fn from_str(s: &str) -> std::result::Result<HexField<WIDTH>, Self::Err> {
        Ok(HexField::<WIDTH>(u64::from_str_radix(s, 16)?))
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

#[derive(Debug, Serialize)]
pub struct RouteList {
    routes: Vec<Route>,
}

impl RouteList {
    pub fn write_all(&self, out_dir: &PathBuf) -> Result<()> {
        for route in &self.routes {
            route.write(out_dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Route {
    id: String,
    #[serde(deserialize_with = "HexField::<6>::deserialize")]
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
    pub fn from_file(dir: &PathBuf, id: String) -> Result<Self> {
        let filename = dir.join(id).with_extension("json");
        let data = fs::read_to_string(filename).expect("Could not read file");
        let listing: Self = serde_json::from_str(&data)?;
        Ok(listing)
    }

    pub fn filename(&self) -> String {
        format!("{}.json", self.id.clone())
    }

    pub fn write(&self, out_dir: &PathBuf) -> Result<()> {
        let data = serde_json::to_string_pretty(&self)?;
        let filename = out_dir.join(self.filename());
        fs::write(filename, data).expect("unable to write file");
        Ok(())
    }

    pub fn remove(&self, out_dir: &PathBuf) -> Result<()> {
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DevaddrRange {
    #[serde(deserialize_with = "HexField::<8>::deserialize")]
    start_addr: HexField<8>,
    #[serde(deserialize_with = "HexField::<8>::deserialize")]
    end_addr: HexField<8>,
}

impl DevaddrRange {
    pub fn new(start_addr: &str, end_addr: &str) -> Result<Self> {
        verify_len("start_addr", start_addr, 8)?;
        verify_len("end_addr", end_addr, 8)?;

        Ok(Self {
            start_addr: HexField(u64::from_str_radix(start_addr, 16)?),
            end_addr: HexField(u64::from_str_radix(end_addr, 16)?),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Eui {
    #[serde(deserialize_with = "HexField::<16>::deserialize")]
    app_eui: HexField<16>,
    #[serde(deserialize_with = "HexField::<16>::deserialize")]
    dev_eui: HexField<16>,
}

impl Eui {
    pub fn new(app_eui: &str, dev_eui: &str) -> Result<Self> {
        verify_len("dev_eui", dev_eui, 16)?;
        verify_len("app_eui", app_eui, 16)?;

        Ok(Self {
            app_eui: HexField(u64::from_str_radix(app_eui, 16)?),
            dev_eui: HexField(u64::from_str_radix(dev_eui, 16)?),
        })
    }
}

fn verify_len(name: &str, input: &str, expected_len: usize) -> Result<()> {
    match input.len() {
        len if len == expected_len => Ok(()),
        len => Err(format!(
            "{name} is {len} chars long, should be {expected_len}"
        ))?,
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
            net_id: route.net_id.into_inner(),
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
    fn hex_net_id_field() {
        let field = &HexField::<6>(0xC00053);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(6 + 2, val.len());
        assert_eq!(r#""C00053""#.to_string(), val);
    }

    #[test]
    fn hex_devaddr_field() {
        let field = &HexField::<8>(0x22ab);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(8 + 2, val.len());
        assert_eq!(r#""000022AB""#.to_string(), val);
    }

    #[test]
    fn hex_eui_field() {
        let field = &HexField::<16>(0x0ABD_68FD_E91E_E0DB);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(16 + 2, val.len());
        assert_eq!(r#""0ABD68FDE91EE0DB""#.to_string(), val)
    }

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
