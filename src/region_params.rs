use crate::Result;
use anyhow::{anyhow, Context};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{fmt, fs, path::PathBuf, str::FromStr};

pub mod proto {
    pub use helium_proto::{
        BlockchainRegionParamV1, BlockchainRegionParamsV1, BlockchainRegionSpreadingV1,
        RegionSpreading, TaggedSpreading,
    };
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionParams {
    pub region_params: Vec<RegionParam>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegionParam {
    pub channel_frequency: u64,
    pub bandwidth: u32,
    pub max_eirp: u32,
    pub spreading: BlockchainRegionSpreading,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockchainRegionSpreading {
    pub tagged_spreading: Vec<TaggedSpreading>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TaggedSpreading {
    pub region_spreading: RegionSpreading,
    pub max_packet_size: u32,
}

#[derive(Clone, Debug)]
pub enum RegionSpreading {
    SfInvalid,
    Sf7,
    Sf8,
    Sf9,
    Sf10,
    Sf11,
    Sf12,
}

impl RegionParams {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let data = fs::read_to_string(path).context("reading params file")?;
        let listing: Self = serde_json::from_str(&data)
            .context(format!("parsing params file {}", path.display()))?;
        Ok(listing)
    }
}

impl From<RegionParams> for proto::BlockchainRegionParamsV1 {
    fn from(rp: RegionParams) -> Self {
        Self {
            region_params: rp.region_params.into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<proto::BlockchainRegionParamsV1> for RegionParams {
    fn from(brp: proto::BlockchainRegionParamsV1) -> Self {
        Self {
            region_params: brp.region_params.into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<RegionParam> for proto::BlockchainRegionParamV1 {
    fn from(rp: RegionParam) -> Self {
        Self {
            channel_frequency: rp.channel_frequency,
            bandwidth: rp.bandwidth,
            max_eirp: rp.max_eirp,
            spreading: Some(rp.spreading.into()),
        }
    }
}

impl From<proto::BlockchainRegionParamV1> for RegionParam {
    fn from(brp: proto::BlockchainRegionParamV1) -> Self {
        Self {
            channel_frequency: brp.channel_frequency,
            bandwidth: brp.bandwidth,
            max_eirp: brp.max_eirp,
            spreading: brp.spreading.unwrap().into(),
        }
    }
}

impl From<BlockchainRegionSpreading> for proto::BlockchainRegionSpreadingV1 {
    fn from(brs: BlockchainRegionSpreading) -> Self {
        Self {
            tagged_spreading: brs
                .tagged_spreading
                .into_iter()
                .map(|ts| ts.into())
                .collect(),
        }
    }
}

impl From<proto::BlockchainRegionSpreadingV1> for BlockchainRegionSpreading {
    fn from(brs: proto::BlockchainRegionSpreadingV1) -> Self {
        Self {
            tagged_spreading: brs
                .tagged_spreading
                .into_iter()
                .map(|ts| ts.into())
                .collect(),
        }
    }
}

impl From<TaggedSpreading> for proto::TaggedSpreading {
    fn from(ts: TaggedSpreading) -> Self {
        Self {
            region_spreading: ts.region_spreading.into(),
            max_packet_size: ts.max_packet_size,
        }
    }
}

impl From<proto::TaggedSpreading> for TaggedSpreading {
    fn from(ts: proto::TaggedSpreading) -> Self {
        Self {
            region_spreading: RegionSpreading::from_i32(ts.region_spreading).unwrap(),
            max_packet_size: ts.max_packet_size,
        }
    }
}

impl RegionSpreading {
    pub fn from_i32(v: i32) -> Result<Self> {
        proto::RegionSpreading::try_from(v)
            .map(|rs| rs.into())
            .map_err(|e| anyhow!("unsupported region spreading {v}: {e:?}"))
    }
}

impl Serialize for RegionSpreading {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let a = proto::RegionSpreading::from(self);
        serializer.serialize_str(&format!("{a}"))
    }
}

impl<'de> Deserialize<'de> for RegionSpreading {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RegionSpreadingVisitor;

        impl<'de> de::Visitor<'de> for RegionSpreadingVisitor {
            type Value = RegionSpreading;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("region spreading string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<RegionSpreading, E>
            where
                E: de::Error,
            {
                let proto_region_spreading =
                    proto::RegionSpreading::from_str(value).map_err(|_| {
                        de::Error::custom(format!("unsupported region spreading: {value}"))
                    })?;
                Ok(proto_region_spreading.into())
            }
        }

        deserializer.deserialize_str(RegionSpreadingVisitor)
    }
}

impl From<RegionSpreading> for proto::RegionSpreading {
    fn from(region: RegionSpreading) -> Self {
        proto::RegionSpreading::from(&region)
    }
}

impl From<&RegionSpreading> for proto::RegionSpreading {
    fn from(sr: &RegionSpreading) -> Self {
        match sr {
            RegionSpreading::SfInvalid => proto::RegionSpreading::SfInvalid,
            RegionSpreading::Sf7 => proto::RegionSpreading::Sf7,
            RegionSpreading::Sf8 => proto::RegionSpreading::Sf8,
            RegionSpreading::Sf9 => proto::RegionSpreading::Sf9,
            RegionSpreading::Sf10 => proto::RegionSpreading::Sf10,
            RegionSpreading::Sf11 => proto::RegionSpreading::Sf11,
            RegionSpreading::Sf12 => proto::RegionSpreading::Sf12,
        }
    }
}

impl From<proto::RegionSpreading> for RegionSpreading {
    fn from(rs: proto::RegionSpreading) -> Self {
        match rs {
            proto::RegionSpreading::SfInvalid => RegionSpreading::SfInvalid,
            proto::RegionSpreading::Sf7 => RegionSpreading::Sf7,
            proto::RegionSpreading::Sf8 => RegionSpreading::Sf8,
            proto::RegionSpreading::Sf9 => RegionSpreading::Sf9,
            proto::RegionSpreading::Sf10 => RegionSpreading::Sf10,
            proto::RegionSpreading::Sf11 => RegionSpreading::Sf11,
            proto::RegionSpreading::Sf12 => RegionSpreading::Sf12,
        }
    }
}

impl From<RegionSpreading> for i32 {
    fn from(region: RegionSpreading) -> Self {
        proto::RegionSpreading::from(region) as i32
    }
}
