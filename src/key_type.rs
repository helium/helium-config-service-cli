use crate::Result;
use anyhow::anyhow;
use helium_proto::services::iot_config::admin_add_key_req_v1::KeyTypeV1 as ProtoKeyType;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};

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

impl Serialize for KeyType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{self}"))
    }
}

impl<'de> Deserialize<'de> for KeyType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyTypeVisitor;

        impl<'de> de::Visitor<'de> for KeyTypeVisitor {
            type Value = KeyType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("key_type string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<KeyType, E>
            where
                E: de::Error,
            {
                KeyType::from_str(value)
                    .map_err(|_| de::Error::custom(format!("unsupported key type: {value}")))
            }
        }

        deserializer.deserialize_str(KeyTypeVisitor)
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
