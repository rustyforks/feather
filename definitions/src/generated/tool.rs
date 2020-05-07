// This file is @generated
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ToPrimitive, FromPrimitive)]
pub enum Tool {
    Axe,
    Pickaxe,
    Shovel,
    Hoe,
    Sword,
    Shears,
}
impl Tool {}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ToPrimitive, FromPrimitive)]
pub enum ToolMaterial {
    Wood,
    Stone,
    Iron,
    Diamond,
    Gold,
}
impl ToolMaterial {
    pub fn dig_multiplier(self) -> f64 {
        use ToolMaterial::*;
        match self {
            Diamond => 8f64,
            Gold => 12f64,
            Iron => 6f64,
            Stone => 4f64,
            Wood => 2f64,
        }
    }
}
