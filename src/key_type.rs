use crate::Result;
use anyhow::anyhow;
use helium_proto::services::iot_config::admin_add_key_req_v1::KeyTypeV1 as ProtoKeyType;
use std::str::FromStr;

#[derive(clap::ValueEnum, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[clap(rename_all = "snake_case")]
pub enum KeyType {
    Operator,
    PacketRouter,
}

impl KeyType {
    pub fn from_i32(v: i32) -> Result<Self> {
        ProtoKeyType::from_i32(v)
            .map(|kt| kt.into())
            .ok_or_else(|| anyhow!("unsupported key type {v}"))
    }
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            KeyType::Operator => f.write_str("operator"),
            KeyType::PacketRouter => f.write_str("packet_router"),
        }
    }
}

impl FromStr for KeyType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "operator" => Ok(KeyType::Operator),
            "packet_router" => Ok(KeyType::PacketRouter),
            _ => Err(anyhow!("invalid key type {s}")),
        }
    }
}

impl From<KeyType> for ProtoKeyType {
    fn from(key_type: KeyType) -> Self {
        ProtoKeyType::from(&key_type)
    }
}

impl From<&KeyType> for ProtoKeyType {
    fn from(skt: &KeyType) -> Self {
        match skt {
            KeyType::Operator => ProtoKeyType::Operator,
            KeyType::PacketRouter => ProtoKeyType::PacketRouter,
        }
    }
}

impl From<ProtoKeyType> for KeyType {
    fn from(kt: ProtoKeyType) -> Self {
        match kt {
            ProtoKeyType::Operator => KeyType::Operator,
            ProtoKeyType::PacketRouter => KeyType::PacketRouter,
        }
    }
}

impl From<KeyType> for i32 {
    fn from(key_type: KeyType) -> Self {
        ProtoKeyType::from(key_type) as i32
    }
}
