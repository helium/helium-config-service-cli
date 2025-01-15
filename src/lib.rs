pub mod clients;
pub mod cmds;
pub mod helium_netids;
pub mod hex_field;
pub mod lora_field;
pub mod region;
pub mod region_params;
pub mod route;
pub mod server;
pub mod subnet;

use anyhow::{anyhow, Error};
use route::Route;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use solana_sdk::pubkey::Pubkey;
use std::fmt::Display;
use subnet::DevaddrConstraint;

pub mod proto {
    pub use helium_proto::services::iot_config::{
        admin_add_key_req_v1::KeyTypeV1, route_skf_update_req_v1::RouteSkfUpdateV1, ActionV1,
        DevaddrConstraintV1, DevaddrRangeV1, EuiPairV1, GatewayLocationResV1, OrgEnableResV1,
        OrgListResV2, OrgResV2, OrgV2, RouteListResV1, SkfV1,
    };
}

pub type Result<T = (), E = Error> = anyhow::Result<T, E>;

type Oui = u64;
type NetId = u32;

#[derive(Debug, Serialize)]
pub enum Msg {
    DryRun(String),
    Success(String),
    Error(String),
}

impl Msg {
    pub fn ok(msg: String) -> Result<Self> {
        Ok(Self::Success(msg))
    }
    pub fn err(msg: String) -> Result<Self> {
        Ok(Self::Error(msg))
    }
    pub fn dry_run(msg: String) -> Result<Self> {
        Ok(Self::DryRun(msg))
    }
    pub fn into_inner(self) -> String {
        match self {
            Msg::DryRun(s) => s,
            Msg::Success(s) => s,
            Msg::Error(s) => s,
        }
    }
}

impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::DryRun(msg) => write!(f, "== DRY RUN == (pass `--commit`)\n{msg}"),
            Msg::Success(msg) => write!(f, "{msg}"),
            Msg::Error(msg) => write!(f, "\u{2717} {msg}"),
        }
    }
}

pub trait PrettyJson {
    fn print_pretty_json(&self) -> Result;
    fn pretty_json(&self) -> Result<String>;
}

impl<S: ?Sized + serde::Serialize> PrettyJson for S {
    fn print_pretty_json(&self) -> Result {
        println!("{}", self.pretty_json()?);
        Ok(())
    }

    fn pretty_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).map_err(|e| e.into())
    }
}

#[derive(Debug, Serialize)]
pub struct OrgResponse {
    pub org: Org,
    pub net_id: hex_field::HexNetID,
    pub devaddr_constraints: Vec<DevaddrConstraint>,
}

impl From<proto::OrgResV2> for OrgResponse {
    fn from(res: proto::OrgResV2) -> Self {
        Self {
            org: res.org.expect("no org returned during creation").into(),
            net_id: hex_field::net_id(res.net_id),
            devaddr_constraints: res
                .devaddr_constraints
                .into_iter()
                .map(|d| d.into())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrgList {
    pub orgs: Vec<Org>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize)]
pub struct Org {
    pub oui: Oui,
    #[serde_as(as = "DisplayFromStr")]
    pub address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: Pubkey,
    pub escrow_key: String,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub delegate_keys: Vec<Pubkey>,
    pub approved: bool,
    pub locked: bool,
}

#[derive(Debug, Serialize)]
pub struct RouteList {
    pub routes: Vec<Route>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Hash)]
pub struct DevaddrRange {
    pub route_id: String,
    pub start_addr: hex_field::HexDevAddr,
    pub end_addr: hex_field::HexDevAddr,
}

impl DevaddrRange {
    pub fn new(
        route_id: String,
        start_addr: hex_field::HexDevAddr,
        end_addr: hex_field::HexDevAddr,
    ) -> Result<Self> {
        if end_addr < start_addr {
            return Err(anyhow!("start_addr cannot be greater than end_addr"));
        }

        Ok(Self {
            route_id,
            start_addr,
            end_addr,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Eui {
    pub route_id: String,
    pub app_eui: hex_field::HexEui,
    pub dev_eui: hex_field::HexEui,
}

impl Eui {
    pub fn new(
        route_id: String,
        app_eui: hex_field::HexEui,
        dev_eui: hex_field::HexEui,
    ) -> Result<Self> {
        Ok(Self {
            route_id,
            app_eui,
            dev_eui,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Skf {
    pub route_id: String,
    pub devaddr: hex_field::HexDevAddr,
    pub session_key: String,
    pub max_copies: Option<u32>,
}

impl Skf {
    pub fn new(
        route_id: String,
        devaddr: hex_field::HexDevAddr,
        session_key: String,
        max_copies: Option<u32>,
    ) -> Result<Self> {
        Ok(Self {
            route_id,
            devaddr,
            session_key,
            max_copies,
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum UpdateAction {
    #[serde(alias = "add")]
    Add,
    #[serde(alias = "remove")]
    Remove,
}

#[derive(Debug, Deserialize)]
pub struct SkfUpdate {
    pub devaddr: hex_field::HexDevAddr,
    pub session_key: String,
    pub action: UpdateAction,
    pub max_copies: Option<u32>,
}

impl From<SkfUpdate> for proto::RouteSkfUpdateV1 {
    fn from(update: SkfUpdate) -> Self {
        let action = match update.action {
            UpdateAction::Add => proto::ActionV1::Add,
            UpdateAction::Remove => proto::ActionV1::Remove,
        }
        .into();

        Self {
            devaddr: update.devaddr.into(),
            session_key: update.session_key,
            action,
            max_copies: update.max_copies.unwrap_or(1),
        }
    }
}

#[derive(Debug, clap::ValueEnum, Clone, Copy)]
pub enum KeyType {
    Administrator,
    PacketRouter,
    Oracle,
}

impl From<KeyType> for proto::KeyTypeV1 {
    fn from(value: KeyType) -> Self {
        match value {
            KeyType::Administrator => Self::Administrator,
            KeyType::PacketRouter => Self::PacketRouter,
            KeyType::Oracle => Self::Oracle,
        }
    }
}

impl From<KeyType> for i32 {
    fn from(value: KeyType) -> Self {
        proto::KeyTypeV1::from(value) as i32
    }
}

impl Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Administrator => write!(f, "Administrator"),
            KeyType::PacketRouter => write!(f, "Packet-Router"),
            KeyType::Oracle => write!(f, "Oracle"),
        }
    }
}

impl From<proto::SkfV1> for Skf {
    fn from(filter: proto::SkfV1) -> Self {
        Self {
            route_id: filter.route_id,
            devaddr: (filter.devaddr as u64).into(),
            session_key: filter.session_key,
            max_copies: Some(filter.max_copies),
        }
    }
}

impl From<Skf> for proto::SkfV1 {
    fn from(filter: Skf) -> Self {
        Self {
            route_id: filter.route_id,
            devaddr: filter.devaddr.0 as u32,
            session_key: filter.session_key,
            max_copies: filter.max_copies.unwrap_or(1),
        }
    }
}

impl From<proto::OrgListResV2> for OrgList {
    fn from(org_list: proto::OrgListResV2) -> Self {
        Self {
            orgs: org_list.orgs.into_iter().map(|o| o.into()).collect(),
        }
    }
}

impl From<proto::OrgV2> for Org {
    fn from(org: proto::OrgV2) -> Self {
        let d = org.delegate_keys.into_iter().flat_map(Pubkey::try_from);

        Self {
            oui: org.oui,
            address: Pubkey::try_from(org.address).expect("Invalid address public key"),
            owner: Pubkey::try_from(org.owner).expect("Invalid owner public key"),
            escrow_key: org.escrow_key,
            delegate_keys: d.collect(),
            approved: org.approved,
            locked: org.locked,
        }
    }
}

impl From<Org> for proto::OrgV2 {
    fn from(org: Org) -> Self {
        Self {
            oui: org.oui,
            address: org.address.to_bytes().to_vec(),
            owner: org.owner.to_bytes().to_vec(),
            escrow_key: org.escrow_key,
            delegate_keys: org
                .delegate_keys
                .iter()
                .map(|key| key.to_bytes().to_vec())
                .collect(),
            approved: org.approved,
            locked: org.locked,
        }
    }
}

impl From<proto::RouteListResV1> for RouteList {
    fn from(route_list: proto::RouteListResV1) -> Self {
        Self {
            routes: route_list.routes.into_iter().map(|r| r.into()).collect(),
        }
    }
}

impl From<proto::DevaddrRangeV1> for DevaddrRange {
    fn from(range: proto::DevaddrRangeV1) -> Self {
        Self {
            route_id: range.route_id,
            start_addr: range.start_addr.into(),
            end_addr: range.end_addr.into(),
        }
    }
}

impl From<&proto::DevaddrRangeV1> for DevaddrRange {
    fn from(range: &proto::DevaddrRangeV1) -> Self {
        Self {
            route_id: range.route_id.to_owned(),
            start_addr: range.start_addr.into(),
            end_addr: range.end_addr.into(),
        }
    }
}

impl From<DevaddrRange> for proto::DevaddrRangeV1 {
    fn from(range: DevaddrRange) -> Self {
        Self {
            route_id: range.route_id,
            start_addr: range.start_addr.into(),
            end_addr: range.end_addr.into(),
        }
    }
}

impl From<proto::DevaddrConstraintV1> for DevaddrConstraint {
    fn from(value: proto::DevaddrConstraintV1) -> Self {
        Self {
            start_addr: value.start_addr.into(),
            end_addr: value.end_addr.into(),
        }
    }
}

impl From<DevaddrConstraint> for proto::DevaddrConstraintV1 {
    fn from(value: DevaddrConstraint) -> Self {
        Self {
            start_addr: value.start_addr.into(),
            end_addr: value.end_addr.into(),
        }
    }
}

impl From<proto::EuiPairV1> for Eui {
    fn from(value: proto::EuiPairV1) -> Self {
        Self {
            route_id: value.route_id,
            app_eui: value.app_eui.into(),
            dev_eui: value.dev_eui.into(),
        }
    }
}

impl From<&proto::EuiPairV1> for Eui {
    fn from(value: &proto::EuiPairV1) -> Self {
        Self {
            route_id: value.route_id.clone(),
            app_eui: value.app_eui.into(),
            dev_eui: value.dev_eui.into(),
        }
    }
}

impl From<Eui> for proto::EuiPairV1 {
    fn from(value: Eui) -> Self {
        Self {
            route_id: value.route_id,
            app_eui: value.app_eui.0,
            dev_eui: value.dev_eui.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hex_field, DevaddrRange, Eui};

    #[test]
    fn deserialize_devaddr_range() {
        let d = r#"{"route_id": "the-route-id", "start_addr": "11223344", "end_addr": "22334455"}"#;
        let val: DevaddrRange = serde_json::from_str(d).unwrap();
        assert_eq!(
            DevaddrRange {
                route_id: "the-route-id".to_string(),
                start_addr: hex_field::devaddr(0x11223344),
                end_addr: hex_field::devaddr(0x22334455)
            },
            val
        );
    }

    #[test]
    fn deserialize_eui() {
        let d = r#"{"route_id": "the-route-id", "app_eui": "1122334411223344", "dev_eui": "2233445522334455"}"#;
        let val: Eui = serde_json::from_str(d).unwrap();
        assert_eq!(
            Eui {
                route_id: "the-route-id".to_string(),
                app_eui: hex_field::eui(0x1122334411223344),
                dev_eui: hex_field::eui(0x2233445522334455)
            },
            val
        );
    }
}
