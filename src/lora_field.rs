#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LoraField<const WIDTH: usize>(pub u64);

pub type NetIdField = LoraField<6>;

impl<const WIDTH: usize> From<LoraField<WIDTH>> for u64 {
    fn from(field: LoraField<WIDTH>) -> Self {
        field.0
    }
}

impl<const WIDTH: usize> From<u64> for LoraField<WIDTH> {
    fn from(val: u64) -> Self {
        LoraField(val)
    }
}
