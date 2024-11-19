use crate::lora_field::{LoraField, NetIdField};
use crate::proto;
use std::ops::RangeInclusive;

const TYPE_0_ID: NetIdField = LoraField(0x00003c);
const TYPE_3_ID: NetIdField = LoraField(0x60002d);
const TYPE_6_ID: NetIdField = LoraField(0xc00053);
const TYPE_0_RANGE: RangeInclusive<u32> = 2_013_265_920..=2_046_820_351;
const TYPE_3_RANGE: RangeInclusive<u32> = 3_763_994_624..=3_764_125_695;
const TYPE_6_RANGE: RangeInclusive<u32> = 4_227_943_424..=4_227_944_447;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum HeliumNetId {
    #[value(alias("0x00003c"))]
    Type0_0x00003c,
    #[value(alias("0x60002d"))]
    Type3_0x60002d,
    #[value(alias("0xc00053"))]
    Type6_0xc00053,
}

impl From<HeliumNetId> for proto::HeliumNetId {
    fn from(id: HeliumNetId) -> Self {
        match id {
            HeliumNetId::Type0_0x00003c => proto::HeliumNetId::Type00x00003c,
            HeliumNetId::Type3_0x60002d => proto::HeliumNetId::Type30x60002d,
            HeliumNetId::Type6_0xc00053 => proto::HeliumNetId::Type60xc00053,
        }
    }
}

impl HeliumNetId {
    pub fn id(&self) -> NetIdField {
        match *self {
            HeliumNetId::Type0_0x00003c => TYPE_0_ID,
            HeliumNetId::Type3_0x60002d => TYPE_3_ID,
            HeliumNetId::Type6_0xc00053 => TYPE_6_ID,
        }
    }

    pub fn addr_range(&self) -> RangeInclusive<u32> {
        match *self {
            HeliumNetId::Type0_0x00003c => TYPE_0_RANGE,
            HeliumNetId::Type3_0x60002d => TYPE_3_RANGE,
            HeliumNetId::Type6_0xc00053 => TYPE_6_RANGE,
        }
    }
}

impl TryFrom<NetIdField> for HeliumNetId {
    type Error = &'static str;

    fn try_from(field: NetIdField) -> Result<Self, Self::Error> {
        let id = match field {
            TYPE_0_ID => HeliumNetId::Type0_0x00003c,
            TYPE_3_ID => HeliumNetId::Type3_0x60002d,
            TYPE_6_ID => HeliumNetId::Type6_0xc00053,
            _ => return Err("not a helium id"),
        };
        Ok(id)
    }
}
