use crate::Result;
use anyhow::anyhow;
use helium_proto::Region as ProtoRegion;
use serde::{de, Deserialize, Deserializer, Serialize};
#[allow(unused_imports)]
use std::{
    fmt::{self, Display},
    str::FromStr,
};

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[clap(rename_all = "snake_case")]
pub enum SupportedRegion {
    Us915,
    Eu868,
    Eu433,
    Cn470,
    Cn779,
    Au915,
    As923_1,
    As923_1b,
    As923_2,
    As923_3,
    As923_4,
    Kr920,
    In865,
    Cd900_1a,
}

impl SupportedRegion {
    pub fn from_i32(v: i32) -> Result<Self> {
        ProtoRegion::from_i32(v)
            .map(|r| r.into())
            .ok_or_else(|| anyhow!("unsupported region {v}"))
    }
}

impl Serialize for SupportedRegion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let a = ProtoRegion::from(self);
        serializer.serialize_str(&format!("{a}"))
    }
}

impl<'de> Deserialize<'de> for SupportedRegion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RegionVisitor;

        impl<'de> de::Visitor<'de> for RegionVisitor {
            type Value = SupportedRegion;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("region string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<SupportedRegion, E>
            where
                E: de::Error,
            {
                let proto_region = ProtoRegion::from_str(value)
                    .map_err(|_| de::Error::custom(format!("unsupported region: {value}")))?;
                Ok(proto_region.into())
            }
        }

        deserializer.deserialize_str(RegionVisitor)
    }
}

impl From<SupportedRegion> for ProtoRegion {
    fn from(region: SupportedRegion) -> Self {
        ProtoRegion::from(&region)
    }
}

impl From<&SupportedRegion> for ProtoRegion {
    fn from(sr: &SupportedRegion) -> Self {
        match sr {
            SupportedRegion::Us915 => ProtoRegion::Us915,
            SupportedRegion::Eu868 => ProtoRegion::Eu868,
            SupportedRegion::Eu433 => ProtoRegion::Eu433,
            SupportedRegion::Cn470 => ProtoRegion::Cn470,
            SupportedRegion::Cn779 => ProtoRegion::Cn779,
            SupportedRegion::Au915 => ProtoRegion::Au915,
            SupportedRegion::As923_1 => ProtoRegion::As9231,
            SupportedRegion::As923_1b => ProtoRegion::As9231b,
            SupportedRegion::As923_2 => ProtoRegion::As9232,
            SupportedRegion::As923_3 => ProtoRegion::As9233,
            SupportedRegion::As923_4 => ProtoRegion::As9234,
            SupportedRegion::Kr920 => ProtoRegion::Kr920,
            SupportedRegion::In865 => ProtoRegion::In865,
            SupportedRegion::Cd900_1a => ProtoRegion::Cd9001a,
        }
    }
}

impl From<ProtoRegion> for SupportedRegion {
    fn from(r: ProtoRegion) -> Self {
        match r {
            ProtoRegion::Us915 => SupportedRegion::Us915,
            ProtoRegion::Eu868 => SupportedRegion::Eu868,
            ProtoRegion::Eu433 => SupportedRegion::Eu433,
            ProtoRegion::Cn470 => SupportedRegion::Cn470,
            ProtoRegion::Cn779 => SupportedRegion::Cn779,
            ProtoRegion::Au915 => SupportedRegion::Au915,
            ProtoRegion::As9231 => SupportedRegion::As923_1,
            ProtoRegion::As9231b => SupportedRegion::As923_1b,
            ProtoRegion::As9232 => SupportedRegion::As923_2,
            ProtoRegion::As9233 => SupportedRegion::As923_3,
            ProtoRegion::As9234 => SupportedRegion::As923_4,
            ProtoRegion::Kr920 => SupportedRegion::Kr920,
            ProtoRegion::In865 => SupportedRegion::In865,
            ProtoRegion::Cd9001a => SupportedRegion::Cd900_1a,
        }
    }
}

impl From<SupportedRegion> for i32 {
    fn from(region: SupportedRegion) -> Self {
        ProtoRegion::from(region) as i32
    }
}

#[cfg(test)]
mod tests {
    use crate::region::SupportedRegion;
    use std::collections::BTreeMap;

    #[test]
    fn hashmap_supported_region_ser() {
        let a = BTreeMap::from([(SupportedRegion::As923_1, "one")]);
        let s = serde_json::to_string_pretty(&a).unwrap();
        println!("{}", s);
        let b: BTreeMap<SupportedRegion, &str> = serde_json::from_str(&s).unwrap();
        println!("{b:?}");
    }
}
