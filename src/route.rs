use crate::{
    hex_field,
    server::{GwmpMap, Http, Server},
    Oui, Result,
};
use helium_proto::services::iot_config::RouteV1 as ProtoRoute;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    pub id: String,
    pub net_id: hex_field::HexNetID,
    pub oui: Oui,
    pub server: Server,
    pub max_copies: u32,
    pub active: bool,
    pub locked: bool,
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
        };
        assert_eq!(route, Route::from(v1.clone()));
        assert_eq!(v1, RouteV1::from(route));
    }
}
