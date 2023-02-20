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

pub type MaterialID = String;

#[derive(Debug)]
pub struct Material {
    pub display_name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MaterialInstance {
    pub material_id: MaterialID,
    pub physics: PhysicsType,
    pub color: Color,
}

impl MaterialInstance {
    #[inline]
    pub fn air() -> Self {
        Self {
            material_id: AIR.to_string(),
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

pub const AIR: &str = "air";
pub const TEST: &str = "test";

pub const COBBLE_STONE: &str = "cobble_stone";
pub const COBBLE_DIRT: &str = "cobble_dirt";
pub const FADED_COBBLE_STONE: &str = "faded_cobble_stone";
pub const FADED_COBBLE_DIRT: &str = "faded_cobble_dirt";
pub const SMOOTH_STONE: &str = "smooth_stone";
pub const SMOOTH_DIRT: &str = "smooth_dirt";

pub const STRUCTURE_VOID: &str = "structure_void";

pub type MaterialRegistry = Registry<MaterialID, Material>;

pub fn init_material_types() -> MaterialRegistry {
    let mut registry = Registry::new();

    registry.register(
        AIR.to_string(),
        Material { display_name: "Air".to_string() },
    );
    registry.register(
        TEST.to_string(),
        Material { display_name: "Test".to_string() },
    );
    registry.register(
        COBBLE_STONE.to_string(),
        Material { display_name: "Cobblestone".to_string() },
    );
    registry.register(
        COBBLE_DIRT.to_string(),
        Material { display_name: "Cobbledirt".to_string() },
    );
    registry.register(
        FADED_COBBLE_STONE.to_string(),
        Material { display_name: "Faded Cobblestone".to_string() },
    );
    registry.register(
        FADED_COBBLE_DIRT.to_string(),
        Material { display_name: "Faded Cobbledirt".to_string() },
    );
    registry.register(
        SMOOTH_STONE.to_string(),
        Material { display_name: "Smoth Stone".to_string() },
    );
    registry.register(
        SMOOTH_DIRT.to_string(),
        Material { display_name: "Dirt".to_string() },
    );
    registry.register(
        STRUCTURE_VOID.to_string(),
        Material { display_name: "Structure Void".to_string() },
    );

    registry
}
