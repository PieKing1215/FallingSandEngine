pub mod color;
pub mod placer;
pub mod registry;

use serde::{Deserialize, Serialize};

use self::{color::Color, registry::Registry};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MaterialInstance {
    pub material_id: u16,
    pub physics: PhysicsType,
    pub color: Color,
}

impl MaterialInstance {
    #[inline]
    pub const fn air() -> Self {
        Self {
            material_id: AIR,
            physics: PhysicsType::Air,
            color: Color::TRANSPARENT,
        }
    }
}

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}

pub const AIR: MaterialID = 0;
pub const TEST: MaterialID = 1;

pub const COBBLE_STONE: MaterialID = 2;
pub const COBBLE_DIRT: MaterialID = 3;
pub const FADED_COBBLE_STONE: MaterialID = 4;
pub const FADED_COBBLE_DIRT: MaterialID = 5;
pub const SMOOTH_STONE: MaterialID = 6;
pub const SMOOTH_DIRT: MaterialID = 7;

pub const STRUCTURE_VOID: MaterialID = 8;

pub type MaterialRegistry = Registry<MaterialID, Material>;

pub fn init_material_types() -> MaterialRegistry {
    let mut registry = Registry::new();

    registry.register(AIR, Material { display_name: "Air".to_string() });
    registry.register(TEST, Material { display_name: "Test".to_string() });
    registry.register(
        COBBLE_STONE,
        Material { display_name: "Cobblestone".to_string() },
    );
    registry.register(
        COBBLE_DIRT,
        Material { display_name: "Cobbledirt".to_string() },
    );
    registry.register(
        FADED_COBBLE_STONE,
        Material { display_name: "Faded Cobblestone".to_string() },
    );
    registry.register(
        FADED_COBBLE_DIRT,
        Material { display_name: "Faded Cobbledirt".to_string() },
    );
    registry.register(
        SMOOTH_STONE,
        Material { display_name: "Smoth Stone".to_string() },
    );
    registry.register(SMOOTH_DIRT, Material { display_name: "Dirt".to_string() });
    registry.register(
        STRUCTURE_VOID,
        Material { display_name: "Structure Void".to_string() },
    );

    registry
}
