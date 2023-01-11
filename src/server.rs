use crate::{region::Region, Result};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod proto {
    pub use helium_proto::services::iot_config::{
        protocol_http_roaming_v1::FlowTypeV1, server_v1::Protocol, ProtocolGwmpMappingV1,
        ProtocolGwmpV1, ProtocolHttpRoamingV1, ProtocolPacketRouterV1, ServerV1,
    };
}

pub type Port = u32;
pub type GwmpMap = BTreeMap<Region, Port>;

#[derive(Serialize, Clone, Debug, Deserialize, PartialEq, Eq, Default)]
pub struct Server {
    pub host: String,
    pub port: Port,
    pub protocol: Option<Protocol>,
}

impl Server {
    pub fn new(host: String, port: Port, protocol: Protocol) -> Self {
        Self {
            host,
            port,
            protocol: Some(protocol),
        }
    }

    pub fn gwmp_add_mapping(&mut self, map: GwmpMap) -> Result {
        if let Some(ref mut p) = self.protocol {
            return p.gwmp_add_mapping(map);
        }

        Err(anyhow!("server has no protocol to update"))
    }

    pub fn http_update(&mut self, http: Http) -> Result {
        if let Some(ref mut p) = self.protocol {
            return p.http_update(http);
        }
        Err(anyhow!("server has no protocol to update"))
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Protocol {
    Gwmp(Gwmp),
    Http(Http),
    PacketRouter,
}

impl Protocol {
    pub fn default_gwmp() -> Self {
        Protocol::Gwmp(Gwmp::default())
    }

    pub fn default_http() -> Self {
        Protocol::Http(Http::default())
    }

    pub fn default_packet_router() -> Self {
        Protocol::PacketRouter
    }

    pub fn make_gwmp_mapping(region: Region, port: Port) -> GwmpMap {
        BTreeMap::from([(region, port)])
    }

    pub fn make_http(
        flow_type: FlowType,
        dedupe_timeout: u32,
        path: String,
        auth_header: Option<String>,
    ) -> Self {
        Self::Http(Http {
            flow_type,
            dedupe_timeout,
            path,
            auth_header: auth_header.unwrap_or_default(),
        })
    }

    pub fn make_gwmp(region: Region, port: Port) -> Result<Self> {
        let mut gwmp = Self::default_gwmp();
        gwmp.gwmp_add_mapping(Self::make_gwmp_mapping(region, port))?;
        Ok(gwmp)
    }

    fn gwmp_add_mapping(&mut self, map: GwmpMap) -> Result {
        match self {
            Protocol::Gwmp(Gwmp { ref mut mapping }) => {
                mapping.extend(map);
                Ok(())
            }
            Protocol::Http(_) => Err(anyhow!("cannot add region mapping to http")),
            Protocol::PacketRouter => Err(anyhow!("cannot add region mapping to packet router")),
        }
    }

    fn http_update(&mut self, http: Http) -> Result {
        match self {
            Protocol::Http(_) => {
                *self = Protocol::Http(http);
                Ok(())
            }
            Protocol::Gwmp(_) => Err(anyhow!("cannot update gwmp with http details")),
            Protocol::PacketRouter => Err(anyhow!("cannot update packet router with http details")),
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct Gwmp {
    pub mapping: GwmpMap,
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct Http {
    flow_type: FlowType,
    dedupe_timeout: u32,
    path: String,
    auth_header: String,
}

#[derive(clap::ValueEnum, Clone, Serialize, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FlowType {
    #[default]
    Sync,
    Async,
}

impl FlowType {
    fn from_i32(v: i32) -> Result<Self> {
        proto::FlowTypeV1::from_i32(v)
            .map(|ft| ft.into())
            .ok_or_else(|| anyhow!("unsupported flow type {v}"))
    }
}

impl From<proto::FlowTypeV1> for FlowType {
    fn from(ft: proto::FlowTypeV1) -> Self {
        match ft {
            proto::FlowTypeV1::Sync => FlowType::Sync,
            proto::FlowTypeV1::Async => FlowType::Async,
        }
    }
}

impl From<FlowType> for proto::FlowTypeV1 {
    fn from(ft: FlowType) -> Self {
        match ft {
            FlowType::Sync => proto::FlowTypeV1::Sync,
            FlowType::Async => proto::FlowTypeV1::Async,
        }
    }
}

impl From<Server> for proto::ServerV1 {
    fn from(server: Server) -> Self {
        proto::ServerV1 {
            host: server.host,
            port: server.port,
            protocol: server.protocol.map(|p| p.into()),
        }
    }
}

impl From<proto::ServerV1> for Server {
    fn from(server: proto::ServerV1) -> Self {
        Server {
            host: server.host,
            port: server.port,
            protocol: server.protocol.map(|p| p.into()),
        }
    }
}

impl From<Protocol> for proto::Protocol {
    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Gwmp(gwmp) => {
                let mut mapping = vec![];
                for (region, port) in gwmp.mapping.into_iter() {
                    mapping.push(proto::ProtocolGwmpMappingV1 {
                        region: region.into(),
                        port,
                    })
                }
                proto::Protocol::Gwmp(proto::ProtocolGwmpV1 { mapping })
            }
            Protocol::Http(http) => proto::Protocol::HttpRoaming(proto::ProtocolHttpRoamingV1 {
                flow_type: proto::FlowTypeV1::from(http.flow_type) as i32,
                dedupe_timeout: http.dedupe_timeout,
                path: http.path,
                auth_header: http.auth_header,
            }),
            Protocol::PacketRouter => {
                proto::Protocol::PacketRouter(proto::ProtocolPacketRouterV1 {})
            }
        }
    }
}

impl From<proto::Protocol> for Protocol {
    fn from(proto: proto::Protocol) -> Self {
        match proto {
            proto::Protocol::Gwmp(gwmp) => {
                let mut mapping = BTreeMap::new();
                for entry in gwmp.mapping {
                    let region = Region::from_i32(entry.region).unwrap();
                    mapping.insert(region, entry.port);
                }
                Protocol::Gwmp(Gwmp { mapping })
            }
            proto::Protocol::HttpRoaming(http) => Protocol::Http(Http {
                flow_type: FlowType::from_i32(http.flow_type).unwrap(),
                dedupe_timeout: http.dedupe_timeout,
                path: http.path,
                auth_header: http.auth_header,
            }),
            proto::Protocol::PacketRouter(_args) => Protocol::PacketRouter,
        }
    }
}

#[cfg(test)]
mod tests {
    /// Ensure all the keys and values are snake_cased.
    /// Serialize regions as lowercase with underscores in the right places.
    use super::{Gwmp, Protocol, Server};
    use crate::{
        region::Region,
        server::{FlowType, Http},
    };
    use serde_test::{assert_ser_tokens, Token};
    use std::collections::BTreeMap;

    #[test]
    fn server_ser() {
        let server = Server {
            host: "example.com".into(),
            port: 1337,
            protocol: None,
        };

        assert_ser_tokens(
            &server,
            &[
                Token::Struct {
                    name: "Server",
                    len: 3,
                },
                Token::Str("host"),
                Token::Str("example.com"),
                Token::Str("port"),
                Token::U32(1337),
                Token::Str("protocol"),
                Token::None,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn gwmp_ser() {
        let gwmp = Protocol::Gwmp(Gwmp {
            mapping: BTreeMap::from([(Region::As923_1, 1), (Region::Us915, 2), (Region::Eu868, 3)]),
        });

        assert_ser_tokens(
            &gwmp,
            &[
                Token::Struct {
                    name: "Gwmp",
                    len: 2,
                },
                Token::Str("type"),
                Token::Str("gwmp"),
                Token::Str("mapping"),
                Token::Map { len: Some(3) },
                Token::Str("US915"),
                Token::U32(2),
                Token::Str("EU868"),
                Token::U32(3),
                Token::Str("AS923_1"),
                Token::U32(1),
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn http_ser() {
        let http = Protocol::Http(Http {
            flow_type: FlowType::Async,
            dedupe_timeout: 777,
            path: "/fns".into(),
            auth_header: "auth-header".to_string(),
        });
        assert_ser_tokens(
            &http,
            &[
                Token::Struct {
                    name: "Http",
                    len: 5,
                },
                Token::Str("type"),
                Token::Str("http"),
                Token::Str("flow_type"),
                Token::UnitVariant {
                    name: "FlowType",
                    variant: "async",
                },
                Token::Str("dedupe_timeout"),
                Token::U32(777),
                Token::Str("path"),
                Token::Str("/fns"),
                Token::Str("auth_header"),
                Token::Str("auth-header"),
                Token::StructEnd,
            ],
        );
    }

    #[test]
    fn packet_router_ser() {
        let packet_router = Protocol::PacketRouter;
        assert_ser_tokens(
            &packet_router,
            &[
                Token::Struct {
                    name: "Protocol",
                    len: 1,
                },
                Token::Str("type"),
                Token::Str("packet_router"),
                Token::StructEnd,
            ],
        );
    }
}
