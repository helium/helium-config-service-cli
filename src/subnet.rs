use anyhow::anyhow;
use ipnet;
use serde::{Deserialize, Serialize};
use std::net;

use crate::{
    hex_field::{self, HexDevAddr},
    DevaddrRange, Result,
};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DevaddrSubnet {
    range: DevaddrConstraint,
    pub subnets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DevaddrConstraint {
    pub start_addr: hex_field::HexDevAddr,
    pub end_addr: hex_field::HexDevAddr,
}

/// Convenience to get subnet masks from an existing DevaddrRange.
///
/// The range is inclusive. (start..=end)
///
/// # Example
///
/// ```
/// use helium_config_service_cli::subnet::DevaddrConstraint;
/// use helium_config_service_cli::hex_field;
///
/// let start = hex_field::devaddr(0x11_22_33_40);
/// let end = hex_field::devaddr(0x11_22_33_47);
/// let range = DevaddrConstraint::new(start, end).unwrap();
/// let subnet = range.to_subnet();
///
/// let expected = vec!["11223340/29".to_string()];
/// assert_eq!(subnet.subnets, expected);
/// ```
impl DevaddrConstraint {
    pub fn new(start_addr: hex_field::HexDevAddr, end_addr: hex_field::HexDevAddr) -> Result<Self> {
        if end_addr < start_addr {
            return Err(anyhow!("start_addr cannot be greater than end_addr"));
        }

        Ok(Self {
            start_addr,
            end_addr,
        })
    }

    pub fn to_subnet(self) -> DevaddrSubnet {
        let start = net::Ipv4Addr::from(self.start_addr.0 as u32);
        let end = net::Ipv4Addr::from(self.end_addr.0 as u32);

        let subnets = ipnet::Ipv4Subnets::new(start, end, 0)
            .map(|net| {
                let hex: HexDevAddr = net.addr().into();
                format!("{hex}/{}", net.prefix_len())
            })
            .collect::<Vec<_>>();

        if subnets.is_empty() {
            DevaddrSubnet {
                range: self,
                subnets: vec!["invalid".to_string()],
            }
        } else {
            DevaddrSubnet {
                range: self,
                subnets,
            }
        }
    }
}

impl From<DevaddrRange> for DevaddrConstraint {
    fn from(value: DevaddrRange) -> Self {
        Self {
            start_addr: value.start_addr,
            end_addr: value.end_addr,
        }
    }
}

impl DevaddrRange {
    pub fn to_subnet(self) -> DevaddrSubnet {
        DevaddrConstraint::from(self).to_subnet()
    }
}

impl HexDevAddr {
    pub fn to_range(self, add: u32) -> DevaddrConstraint {
        // Range includes starting address
        // (start, end]
        let end = (self.0 + (add - 1) as u64).into();
        DevaddrConstraint {
            start_addr: self,
            end_addr: end,
        }
    }
}

impl From<net::Ipv4Addr> for HexDevAddr {
    fn from(addr: net::Ipv4Addr) -> Self {
        let num: u32 = addr.into();
        Self::from(num as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::DevaddrSubnet;
    use crate::{hex_field, subnet::DevaddrConstraint};
    use pretty_assertions::assert_eq;

    #[test]
    fn subnet_prefix() {
        struct DevaddrBlock {
            size: u32,
            mask: u8,
        }
        let blocks = vec![
            DevaddrBlock { size: 8, mask: 29 },
            DevaddrBlock { size: 16, mask: 28 },
            DevaddrBlock { size: 32, mask: 27 },
            DevaddrBlock { size: 64, mask: 26 },
        ];
        for block in blocks {
            assert_eq!(
                vec![format!("48000800/{}", block.mask)],
                hex_field::devaddr(0x48_00_08_00)
                    .to_range(block.size)
                    .to_subnet()
                    .subnets
            );
        }
    }

    #[test]
    fn subnet_mapping() {
        let start = hex_field::devaddr(0x11_22_33_44);
        let end = hex_field::devaddr(0x11_22_33_4c);

        let valid_range = start.to_range(8);
        assert_eq!(
            valid_range.clone().to_subnet(),
            DevaddrSubnet {
                range: valid_range,
                subnets: vec!["11223344/30".to_string(), "11223348/30".to_string()]
            }
        );

        // It's not simple to create a backwards devaddr range.
        let invalid_range = DevaddrConstraint {
            start_addr: end,
            end_addr: start,
        };
        assert_eq!(
            invalid_range.clone().to_subnet(),
            DevaddrSubnet {
                range: invalid_range,
                subnets: vec!["invalid".to_string()]
            }
        )
    }

    #[test]
    fn subnet_display() {
        assert_eq!(
            r#"["48000800/29"]"#,
            format!(
                "{:?}",
                hex_field::devaddr(0x48_00_08_00)
                    .to_range(8)
                    .to_subnet()
                    .subnets
            )
        );

        assert_eq!(
            r#"["480007FF/32", "48000800/30", "48000804/31", "48000806/32"]"#,
            format!(
                "{:?}",
                hex_field::devaddr(0x48_00_07_ff)
                    .to_range(8)
                    .to_subnet()
                    .subnets
            )
        );
    }
}
