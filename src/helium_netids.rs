use crate::lora_field::{LoraField, NetIdField};

const TYPE_0_ID: NetIdField = LoraField(0x00003c);
const TYPE_3_ID: NetIdField = LoraField(0x60002d);
const TYPE_6_ID: NetIdField = LoraField(0xc00053);

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum HeliumNetId {
    #[value(alias("0x00003c"))]
    Type0_0x00003c,
    #[value(alias("0x60002d"))]
    Type3_0x60002d,
    #[value(alias("0xc00053"))]
    Type6_0xc00053,
}

impl HeliumNetId {
    pub fn id(&self) -> NetIdField {
        match *self {
            HeliumNetId::Type0_0x00003c => TYPE_0_ID,
            HeliumNetId::Type3_0x60002d => TYPE_3_ID,
            HeliumNetId::Type6_0xc00053 => TYPE_6_ID,
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
