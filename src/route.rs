use crate::{
    hex_field,
    server::{GwmpMap, Http, Server},
    DevaddrRange, Eui, Result,
};
use anyhow::Context;
use helium_proto::services::config::RouteV1 as ProtoRoute;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    id: String,
    net_id: hex_field::HexNetID,
    pub devaddr_ranges: Vec<DevaddrRange>,
    pub euis: Vec<Eui>,
    oui: u64,
    pub server: Server,
    max_copies: u32,
    nonce: u32,
}

impl Route {
    pub fn new(net_id: hex_field::HexNetID, oui: u64, max_copies: u32) -> Self {
        Self {
            id: "".into(),
            net_id,
            devaddr_ranges: vec![],
            euis: vec![],
            oui,
            server: Server::default(),
            max_copies,
            nonce: 1,
        }
    }
    pub fn from_file<S>(dir: &Path, id: S) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let filename = dir.join(id).with_extension("json");
        let data = fs::read_to_string(&filename)
            .context("reading route file")
            .expect("Could not read file");
        let listing: Self = serde_json::from_str(&data)
            .context(format!("parsing route file {}", filename.display()))?;
        Ok(listing)
    }

    pub fn filename(&self) -> String {
        format!("{}.json", self.id.clone())
    }

    pub fn write(&self, out_dir: &Path) -> Result {
        let data = serde_json::to_string_pretty(&self)?;
        let filename = out_dir.join(self.filename());
        fs::write(filename, data).expect("unable to write file");
        Ok(())
    }

    pub fn remove(&self, out_dir: &Path) -> Result {
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

    pub fn set_server(&mut self, server: Server) {
        self.server = server;
    }

    pub fn gwmp_add_mapping(&mut self, map: GwmpMap) -> Result {
        self.server.gwmp_add_mapping(map)
    }

    pub fn http_update(&mut self, http: Http) -> Result {
        self.server.http_update(http)
    }
}

impl From<ProtoRoute> for Route {
    fn from(route: ProtoRoute) -> Self {
        Self {
            id: String::from_utf8(route.id).unwrap(),
            net_id: route.net_id.into(),
            devaddr_ranges: route.devaddr_ranges.into_iter().map(|r| r.into()).collect(),
            euis: route.euis.into_iter().map(|e| e.into()).collect(),
            oui: route.oui,
            server: route.server.map_or_else(Server::default, |s| s.into()),
            max_copies: route.max_copies,
            nonce: route.nonce,
        }
    }
}

impl From<Route> for ProtoRoute {
    fn from(route: Route) -> Self {
        Self {
            id: route.id.into(),
            net_id: route.net_id.into(),
            devaddr_ranges: route.devaddr_ranges.into_iter().map(|r| r.into()).collect(),
            euis: route.euis.into_iter().map(|e| e.into()).collect(),
            oui: route.oui,
            server: Some(route.server.into()),
            max_copies: route.max_copies,
            nonce: route.nonce,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hex_field, server::Server, DevaddrRange, Eui, Route};
    use helium_proto::services::config::{DevaddrRangeV1, EuiV1, RouteV1, ServerV1};

    #[test]
    fn route_to_route_v1_conversion() {
        let route = Route {
            id: "route_id".into(),
            net_id: hex_field::net_id(1),
            devaddr_ranges: vec![DevaddrRange {
                start_addr: hex_field::devaddr(287454020),
                end_addr: hex_field::devaddr(2005440768),
            }],
            euis: vec![Eui {
                app_eui: hex_field::eui(12302652060662178304),
                dev_eui: hex_field::eui(12302652060662178304),
            }],
            oui: 66,
            server: Server::default(),
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
            server: Some(ServerV1 {
                host: "".into(),
                port: 0,
                protocol: None,
            }),
            max_copies: 999,
            nonce: 1337,
        };
        assert_eq!(route, Route::from(v1.clone()));
        assert_eq!(v1, RouteV1::from(route));
    }
}
