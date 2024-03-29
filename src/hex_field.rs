use crate::{DevaddrConstraint, NetId, Result};
use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
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

impl<const WIDTH: usize> From<HexField<WIDTH>> for u32 {
    fn from(field: HexField<WIDTH>) -> Self {
        field.0 as u32
    }
}

impl<const WIDTH: usize> From<u64> for HexField<WIDTH> {
    fn from(val: u64) -> Self {
        HexField(val)
    }
}

impl<const WIDTH: usize> From<u32> for HexField<WIDTH> {
    fn from(val: u32) -> Self {
        HexField(val as u64)
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
        if "*" == s {
            return Ok(HexField::<WIDTH>(0));
        }
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
                formatter.write_str(&format!("hex string {IN_WIDTH} wide"))
            }

            fn visit_str<E>(self, value: &str) -> anyhow::Result<HexField<IN_WIDTH>, E>
            where
                E: serde::de::Error,
            {
                let field = HexField::<IN_WIDTH>::from_str(value)
                    .map_err(|_| serde::de::Error::invalid_length(value.len(), &self))?;
                Ok(field)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(HexField::<IN_WIDTH>(v))
            }
        }

        deserializer.deserialize_any(HexFieldVisitor::<WIDTH>)
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

pub fn net_id(val: NetId) -> HexNetID {
    val.into()
}

fn verify_len(input: &str, expected_len: usize) -> Result<()> {
    match input.len() {
        len if len == expected_len => Ok(()),
        len => Err(anyhow!("Found {len} chars long, should be {expected_len}"))?,
    }
}

impl HexNetID {
    fn netid_type(&self) -> u32 {
        const BIT_WIDTH: usize = 24;
        const TYPE_LEN: usize = 3;
        let net_id = self.0 as u32;
        net_id >> (BIT_WIDTH - TYPE_LEN)
    }

    fn nwk_id(&self) -> u32 {
        let prefix_length = self.netid_type() + 1;

        let mut temp = self.0 as u32;
        const BIT32PAD: u32 = 8;

        // clear prefix
        temp <<= prefix_length + BIT32PAD;

        // shift to start
        temp >>= prefix_length + BIT32PAD;

        temp
    }

    fn devaddr_type_bits(id_type: u32) -> u32 {
        match id_type {
            0 => 0,
            1 => 2 << (32 - 2),
            2 => 6 << (32 - 3),
            3 => 14 << (32 - 4),
            4 => 30 << (32 - 5),
            5 => 62 << (32 - 6),
            6 => 126 << (32 - 7),
            7 => 254 << (32 - 8),
            _ => panic!("bad type"),
        }
    }

    fn nwk_id_bits(id_type: u32, nwk_id: u32) -> u32 {
        match id_type {
            0 => nwk_id << 25,
            1 => nwk_id << 24,
            2 => nwk_id << 20,
            3 => nwk_id << 17,
            4 => nwk_id << 15,
            5 => nwk_id << 13,
            6 => nwk_id << 10,
            7 => nwk_id << 7,
            _ => panic!("bad type"),
        }
    }

    fn max_nwk_addr_bit(id_type: u32) -> u32 {
        match id_type {
            0 => 2u32.pow(25) - 1,
            1 => 2u32.pow(24) - 1,
            2 => 2u32.pow(20) - 1,
            3 => 2u32.pow(17) - 1,
            4 => 2u32.pow(15) - 1,
            5 => 2u32.pow(13) - 1,
            6 => 2u32.pow(10) - 1,
            7 => 2u32.pow(7) - 1,
            _ => panic!("bad type"),
        }
    }

    pub fn range_start(&self) -> HexDevAddr {
        let id_type = self.netid_type();
        let nwk_id = self.nwk_id();

        let left = Self::devaddr_type_bits(id_type);
        let middle = Self::nwk_id_bits(id_type, nwk_id);

        let min_devaddr = left | middle;
        devaddr(min_devaddr as u64)
    }

    fn range_end(&self) -> HexDevAddr {
        let id_type = self.netid_type();
        let nwk_id = self.nwk_id();

        let left = Self::devaddr_type_bits(id_type);
        let middle = Self::nwk_id_bits(id_type, nwk_id);
        let right = Self::max_nwk_addr_bit(id_type);

        let max_devaddr = left | middle | right;
        devaddr(max_devaddr as u64)
    }

    pub fn full_range(&self) -> DevaddrConstraint {
        DevaddrConstraint {
            start_addr: self.range_start(),
            end_addr: self.range_end(),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use crate::{
        hex_field::{devaddr, eui, net_id},
        DevaddrConstraint, NetId,
    };
    use pretty_assertions::assert_eq;

    use super::HexEui;

    #[test]
    fn range_from_net_id() {
        struct Test {
            net_id: NetId,
            start_addr: u64,
            end_addr: u64,
            netid_type: u32,
            nwk_id: u32,
        }
        let tests = vec![
            Test {
                net_id: 0xC00053,
                start_addr: 0xFC01_4C00,
                end_addr: 0xFC01_4FFF,
                netid_type: 6,
                nwk_id: 83,
            },
            Test {
                net_id: 0x00001D,
                start_addr: 0x3A00_0000,
                end_addr: 0x3BFF_FFFF,
                netid_type: 0,
                nwk_id: 29,
            },
            Test {
                net_id: 0x600020,
                start_addr: 0xE040_0000,
                end_addr: 0xE041_FFFF,
                netid_type: 3,
                nwk_id: 32,
            },
            Test {
                net_id: 0xE00040,
                start_addr: 0xFE00_2000,
                end_addr: 0xFE00_207F,
                netid_type: 7,
                nwk_id: 64,
            },
        ];

        for test in tests {
            let net_id = net_id(test.net_id);
            assert_eq!(test.netid_type, net_id.netid_type());
            assert_eq!(test.nwk_id, net_id.nwk_id());
            assert_eq!(
                DevaddrConstraint::new(devaddr(test.start_addr), devaddr(test.end_addr)).unwrap(),
                net_id.full_range()
            );
        }
    }

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
    fn wildcard_eui_field() {
        let val = HexEui::from_str("*").expect("direct from str");
        assert_eq!(0, val.0);
        let val: HexEui = serde_json::from_str(r#""*""#).expect("serde_json from_str");
        assert_eq!(0, val.0);
    }
}
