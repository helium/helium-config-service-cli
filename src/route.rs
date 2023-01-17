use crate::{
    hex_field,
    server::{GwmpMap, Http, Server},
    Result,
};
use anyhow::Context;
use helium_proto::services::iot_config::RouteV1 as ProtoRoute;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    pub id: String,
    pub net_id: hex_field::HexNetID,
    pub oui: u64,
    pub server: Server,
    pub max_copies: u32,
}

impl Route {
    pub fn new(net_id: hex_field::HexNetID, oui: u64, max_copies: u32) -> Self {
        Self {
            id: "".into(),
            net_id,
            oui,
            server: Server::default(),
            max_copies,
        }
    }
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let data = fs::read_to_string(path).context("reading route file")?;
        let listing: Self = serde_json::from_str(&data)
            .context(format!("parsing route file {}", path.display()))?;
        Ok(listing)
    }

    pub fn from_dir(dir: &Path) -> Result<Vec<Self>> {
        let mut routes = vec![];

        for entry_result in fs::read_dir(dir)? {
            let route = Self::from_file(&entry_result?.path())?;
            routes.push(route);
        }

        Ok(routes)
    }

    pub fn from_id<S>(dir: &Path, id: S) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let filename = dir.join(id).with_extension("json");
        Self::from_file(&filename)
    }

    pub fn filename(&self) -> String {
        format!("{}.json", self.id.clone())
    }

    pub fn write(&self, out: &Path) -> Result {
        // If a directory is passed in, append the filename before continuing
        let out = if out.extension().is_none() {
            out.join(self.filename())
        } else {
            out.into()
        };

        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).context("ensuring parent dir exists")?;
        }

        let data = serde_json::to_string_pretty(&self)?;
        fs::write(out, data).context("writing file")?;
        Ok(())
    }

    pub fn remove(&self, out_dir: &Path) -> Result {
        let filename = out_dir.join(self.filename());
        fs::remove_file(filename)?;
        Ok(())
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
            id: route.id,
            net_id: route.net_id.into(),
            oui: route.oui,
            server: route.server.map_or_else(Server::default, |s| s.into()),
            max_copies: route.max_copies,
        }
    }
}

impl From<Route> for ProtoRoute {
    fn from(route: Route) -> Self {
        Self {
            id: route.id,
            net_id: route.net_id.into(),
            oui: route.oui,
            server: Some(route.server.into()),
            max_copies: route.max_copies,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hex_field, server::Server, Route};
    use helium_proto::services::iot_config::{RouteV1, ServerV1};

    #[test]
    fn route_to_route_v1_conversion() {
        let route = Route {
            id: "route_id".into(),
            net_id: hex_field::net_id(1),
            oui: 66,
            server: Server::default(),
            max_copies: 999,
        };

        let v1 = RouteV1 {
            id: "route_id".into(),
            net_id: 1,
            oui: 66,
            server: Some(ServerV1 {
                host: "".into(),
                port: 0,
                protocol: None,
            }),
            max_copies: 999,
        };
        assert_eq!(route, Route::from(v1.clone()));
        assert_eq!(v1, RouteV1::from(route));
    }
}
