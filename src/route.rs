use crate::{
    hex_field,
    server::{GwmpMap, Http, Server},
    Oui, Result,
};
use helium_proto::services::iot_config::{
    multi_buy_v1, MultiBuyV1, RouteV1 as ProtoRoute,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiBuy {
    pub protocol: MultiBuyProtocol,
    pub host: String,
    pub port: u32,
    pub fail_on_unavailable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum MultiBuyProtocol {
    Http,
    Https,
}

impl From<MultiBuy> for MultiBuyV1 {
    fn from(mb: MultiBuy) -> Self {
        Self {
            protocol: multi_buy_v1::Protocol::from(mb.protocol) as i32,
            host: mb.host,
            port: mb.port,
            fail_on_unavailable: mb.fail_on_unavailable,
        }
    }
}

impl From<MultiBuyV1> for MultiBuy {
    fn from(mb: MultiBuyV1) -> Self {
        Self {
            protocol: MultiBuyProtocol::from_i32(mb.protocol),
            host: mb.host,
            port: mb.port,
            fail_on_unavailable: mb.fail_on_unavailable,
        }
    }
}

impl From<MultiBuyProtocol> for multi_buy_v1::Protocol {
    fn from(p: MultiBuyProtocol) -> Self {
        match p {
            MultiBuyProtocol::Http => Self::Http,
            MultiBuyProtocol::Https => Self::Https,
        }
    }
}

impl MultiBuyProtocol {
    fn from_i32(v: i32) -> Self {
        match multi_buy_v1::Protocol::try_from(v) {
            Ok(multi_buy_v1::Protocol::Https) => Self::Https,
            _ => Self::Http,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    pub id: String,
    pub net_id: hex_field::HexNetID,
    pub oui: Oui,
    pub server: Server,
    pub max_copies: u32,
    pub active: bool,
    pub locked: bool,
    pub ignore_empty_skf: bool,
    pub multi_buy: Option<MultiBuy>,
}

impl Route {
    pub fn new(net_id: hex_field::HexNetID, oui: Oui, max_copies: u32) -> Self {
        Self {
            id: "".into(),
            net_id,
            oui,
            server: Server::default(),
            max_copies,
            locked: false,
            active: true,
            ignore_empty_skf: false,
            multi_buy: None,
        }
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

    pub fn set_ignore_empty_skf(&mut self, ignore: bool) {
        self.ignore_empty_skf = ignore;
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
            locked: route.locked,
            active: route.active,
            ignore_empty_skf: route.ignore_empty_skf,
            multi_buy: route.multi_buy.map(MultiBuy::from),
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
            locked: route.locked,
            active: route.active,
            ignore_empty_skf: route.ignore_empty_skf,
            multi_buy: route.multi_buy.map(MultiBuyV1::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hex_field, server::Server, Route};
    use helium_proto::services::iot_config::{
        server_v1::Protocol, ProtocolPacketRouterV1, RouteV1, ServerV1,
    };

    #[test]
    fn route_to_route_v1_conversion() {
        let route = Route {
            id: "route_id".into(),
            net_id: hex_field::net_id(1),
            oui: 66,
            server: Server::default(),
            max_copies: 999,
            locked: true,
            active: true,
            ignore_empty_skf: false,
            multi_buy: None,
        };

        let v1 = RouteV1 {
            id: "route_id".into(),
            net_id: 1,
            oui: 66,
            server: Some(ServerV1 {
                host: "".into(),
                port: 0,
                protocol: Some(Protocol::PacketRouter(ProtocolPacketRouterV1 {})),
            }),
            max_copies: 999,
            locked: true,
            active: true,
            ignore_empty_skf: false,
            multi_buy: None,
        };
        assert_eq!(route, Route::from(v1.clone()));
        assert_eq!(v1, RouteV1::from(route));
    }
}
