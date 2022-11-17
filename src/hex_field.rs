use crate::Result;
use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HexField<const WIDTH: usize>(pub u64);

pub type HexNetID = HexField<6>;
pub type HexDevAddr = HexField<8>;
pub type HexEui = HexField<16>;

impl<const WIDTH: usize> PartialOrd for HexField<WIDTH> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<const WIDTH: usize> From<HexField<WIDTH>> for u64 {
    fn from(field: HexField<WIDTH>) -> Self {
        field.0
    }
}

impl<const WIDTH: usize> From<u64> for HexField<WIDTH> {
    fn from(val: u64) -> Self {
        HexField(val)
    }
}

impl<const WIDTH: usize> Display for HexField<WIDTH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // pad with 0s to the left up to WIDTH
        write!(f, "{:0>width$X}", self.0, width = WIDTH)
    }
}

impl<const WIDTH: usize> FromStr for HexField<WIDTH> {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<HexField<WIDTH>> {
        verify_len(s, WIDTH)?;
        Ok(HexField::<WIDTH>(u64::from_str_radix(s, 16)?))
    }
}

impl<const WIDTH: usize> Serialize for HexField<WIDTH> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{self}"))
    }
}

impl<'de, const WIDTH: usize> Deserialize<'de> for HexField<WIDTH> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HexFieldVisitor<const IN_WIDTH: usize>;

        impl<'de, const IN_WIDTH: usize> serde::de::Visitor<'de> for HexFieldVisitor<IN_WIDTH> {
            type Value = HexField<IN_WIDTH>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("hex string {} wide", IN_WIDTH))
            }

            fn visit_str<E>(self, value: &str) -> anyhow::Result<HexField<IN_WIDTH>, E>
            where
                E: serde::de::Error,
            {
                let field = HexField::<IN_WIDTH>::from_str(value)
                    .map_err(|_| serde::de::Error::invalid_length(value.len(), &self))?;
                Ok(field)
            }
        }

        deserializer.deserialize_str(HexFieldVisitor::<WIDTH>)
    }
}

impl From<HexDevAddr> for std::net::Ipv4Addr {
    fn from(devaddr: HexDevAddr) -> Self {
        Self::from(devaddr.0 as u32)
    }
}

impl HexDevAddr {
    pub fn subnet_mask(self, end: Self) -> Result<String> {
        let mut subnet = ipnet::Ipv4Subnets::new(self.into(), end.into(), 0);
        let net = subnet.next().expect("end cannot be before start");
        Ok(format!("{self}/{}", net.prefix_len()))
    }
}

pub fn validate_net_id(s: &str) -> Result<HexNetID> {
    HexNetID::from_str(s).map_err(|e| anyhow!("could not parse {s} into net_id, {e}"))
}

pub fn validate_devaddr(s: &str) -> Result<HexDevAddr> {
    HexDevAddr::from_str(s).map_err(|e| anyhow!("could not parse {s} into devaddr, {e}"))
}

pub fn validate_eui(s: &str) -> Result<HexEui> {
    HexEui::from_str(s).map_err(|e| anyhow!("could not parse {s} into eui, {e}"))
}

pub fn devaddr(val: u64) -> HexDevAddr {
    val.into()
}

pub fn eui(val: u64) -> HexEui {
    val.into()
}

pub fn net_id(val: u64) -> HexNetID {
    val.into()
}

fn verify_len(input: &str, expected_len: usize) -> Result<()> {
    match input.len() {
        len if len == expected_len => Ok(()),
        len => Err(anyhow!("Found {len} chars long, should be {expected_len}"))?,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        hex_field::{devaddr, eui, net_id},
        Result,
    };

    #[test]
    fn hex_net_id_field() {
        let field = &net_id(0xC00053);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(6 + 2, val.len());
        assert_eq!(r#""C00053""#.to_string(), val);
    }

    #[test]
    fn hex_devaddr_field() {
        let field = &devaddr(0x22ab);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(8 + 2, val.len());
        assert_eq!(r#""000022AB""#.to_string(), val);
    }

    #[test]
    fn hex_eui_field() {
        let field = &eui(0x0ABD_68FD_E91E_E0DB);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(16 + 2, val.len());
        assert_eq!(r#""0ABD68FDE91EE0DB""#.to_string(), val)
    }

    #[test]
    fn subnet_prefix() -> Result {
        assert_eq!(
            "48000800/29",
            devaddr(0x48_00_08_00).subnet_mask(devaddr(0x48_00_08_07))?
        );
        Ok(())
    }
}
