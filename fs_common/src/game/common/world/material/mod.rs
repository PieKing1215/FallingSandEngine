pub mod color;
pub mod placer;
pub mod registry;

use serde::{Deserialize, Serialize};

use self::{color::Color, registry::Registry};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum PhysicsType {
    Air,
    Solid,
    Sand,
    Liquid,
    Gas,
    Object,
}

pub type MaterialID = u16;

#[derive(Debug)]
pub struct Material {
    pub display_name: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MaterialInstance {
    pub material_id: u16,
    pub physics: PhysicsType,
    pub color: Color,
}

impl MaterialInstance {
    pub fn air() -> Self {
        Self {
            material_id: AIR,
            physics: PhysicsType::Air,
            color: Color::rgba(0, 0, 0, 0),
        }
    }
}

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}

pub static AIR: MaterialID = 0;
pub static TEST: MaterialID = 1;

pub type MaterialRegistry = Registry<MaterialID, Material>;

pub fn init_material_types() -> MaterialRegistry {
    let mut registry = Registry::new();

    registry.register(AIR, Material { display_name: "Air".to_string() });
    registry.register(TEST, Material { display_name: "Test".to_string() });

    registry
}
