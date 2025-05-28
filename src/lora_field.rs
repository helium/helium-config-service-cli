#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LoraField<const WIDTH: usize>(pub u64);

pub type NetIdField = LoraField<6>;

impl<const WIDTH: usize> PartialOrd for LoraField<WIDTH> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

// Freely convert any LoraField to and from 64-bit integers
// but limit conversion between 32-bit integers to the NetId
// and DevAddr fields only
impl<const WIDTH: usize> From<LoraField<WIDTH>> for u64 {
    fn from(field: LoraField<WIDTH>) -> Self {
        field.0
    }
}

impl<const WIDTH: usize> From<LoraField<WIDTH>> for i64 {
    fn from(field: LoraField<WIDTH>) -> Self {
        field.0 as i64
    }
}

impl From<LoraField<6>> for u32 {
    fn from(field: LoraField<6>) -> Self {
        field.0 as u32
    }
}

impl From<LoraField<8>> for u32 {
    fn from(field: LoraField<8>) -> Self {
        field.0 as u32
    }
}

impl From<LoraField<6>> for i32 {
    fn from(field: LoraField<6>) -> Self {
        field.0 as i32
    }
}

impl From<LoraField<8>> for i32 {
    fn from(field: LoraField<8>) -> Self {
        field.0 as i32
    }
}

impl<const WIDTH: usize> From<u64> for LoraField<WIDTH> {
    fn from(val: u64) -> Self {
        LoraField(val)
    }
}
