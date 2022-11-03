use crate::Result;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]
pub struct HexField<const WIDTH: usize>(pub u64);

impl<const WIDTH: usize> From<HexField<WIDTH>> for u64 {
    fn from(field: HexField<WIDTH>) -> Self {
        field.0
    }
}

impl<const WIDTH: usize> Serialize for HexField<WIDTH> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // pad with 0s to the left up to WIDTH
        serializer.serialize_str(&format!("{:0>width$X}", self.0, width = WIDTH))
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

impl<const WIDTH: usize> FromStr for HexField<WIDTH> {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<HexField<WIDTH>> {
        Ok(HexField::<WIDTH>(u64::from_str_radix(s, 16)?))
    }
}

#[cfg(test)]
mod tests {
    use crate::HexField;

    #[test]
    fn hex_net_id_field() {
        let field = &HexField::<6>(0xC00053);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(6 + 2, val.len());
        assert_eq!(r#""C00053""#.to_string(), val);
    }

    #[test]
    fn hex_devaddr_field() {
        let field = &HexField::<8>(0x22ab);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(8 + 2, val.len());
        assert_eq!(r#""000022AB""#.to_string(), val);
    }

    #[test]
    fn hex_eui_field() {
        let field = &HexField::<16>(0x0ABD_68FD_E91E_E0DB);
        let val = serde_json::to_string(field).unwrap();
        // value includes quotes
        assert_eq!(16 + 2, val.len());
        assert_eq!(r#""0ABD68FDE91EE0DB""#.to_string(), val)
    }
}
