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
    Wooden,
    Stone,
    Iron,
    Diamond,
    Gold,
}
impl ToolMaterial {
    pub fn dig_multiplier(self) -> f64 {
        match self {
            ToolMaterial::Diamond => 8f64,
            ToolMaterial::Gold => 12f64,
            ToolMaterial::Iron => 6f64,
            ToolMaterial::Stone => 4f64,
            ToolMaterial::Wooden => 2f64,
        }
    }
}
