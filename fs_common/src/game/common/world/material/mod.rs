pub mod color;
pub mod placer;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::game::common::registry::{Registry, RegistryID};

use self::color::Color;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PhysicsType {
    Air,
    Solid,
    Sand,
    Liquid,
    Gas,
    Object,
}

#[derive(Debug)]
pub struct Material {
    pub display_name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MaterialInstance {
    pub material_id: RegistryID<Material>,
    pub physics: PhysicsType,
    pub color: Color,
}

impl MaterialInstance {
    #[inline]
    pub fn air() -> Self {
        AIR.instance(PhysicsType::Air, Color::TRANSPARENT)
    }
}

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}

impl RegistryID<Material> {
    #[inline]
    pub fn instance(&self, physics: PhysicsType, color: Color) -> MaterialInstance {
        MaterialInstance { material_id: self.clone(), physics, color }
    }
}

pub static AIR: Lazy<RegistryID<Material>> = Lazy::new(|| "air".into());
pub static TEST: Lazy<RegistryID<Material>> = Lazy::new(|| "test".into());

pub static COBBLE_STONE: Lazy<RegistryID<Material>> = Lazy::new(|| "cobble_stone".into());
pub static COBBLE_DIRT: Lazy<RegistryID<Material>> = Lazy::new(|| "cobble_dirt".into());
pub static FADED_COBBLE_STONE: Lazy<RegistryID<Material>> =
    Lazy::new(|| "faded_cobble_stone".into());
pub static FADED_COBBLE_DIRT: Lazy<RegistryID<Material>> = Lazy::new(|| "faded_cobble_dirt".into());
pub static SMOOTH_STONE: Lazy<RegistryID<Material>> = Lazy::new(|| "smooth_stone".into());
pub static SMOOTH_DIRT: Lazy<RegistryID<Material>> = Lazy::new(|| "smooth_dirt".into());

pub static STRUCTURE_VOID: Lazy<RegistryID<Material>> = Lazy::new(|| "structure_void".into());

pub type MaterialRegistry = Registry<Material>;

pub fn init_material_types() -> MaterialRegistry {
    let mut registry = Registry::new();

    registry.register(AIR.clone(), Material { display_name: "Air".to_string() });
    registry.register(TEST.clone(), Material { display_name: "Test".to_string() });
    registry.register(
        COBBLE_STONE.clone(),
        Material { display_name: "Cobblestone".to_string() },
    );
    registry.register(
        COBBLE_DIRT.clone(),
        Material { display_name: "Cobbledirt".to_string() },
    );
    registry.register(
        FADED_COBBLE_STONE.clone(),
        Material { display_name: "Faded Cobblestone".to_string() },
    );
    registry.register(
        FADED_COBBLE_DIRT.clone(),
        Material { display_name: "Faded Cobbledirt".to_string() },
    );
    registry.register(
        SMOOTH_STONE.clone(),
        Material { display_name: "Smoth Stone".to_string() },
    );
    registry.register(
        SMOOTH_DIRT.clone(),
        Material { display_name: "Dirt".to_string() },
    );
    registry.register(
        STRUCTURE_VOID.clone(),
        Material { display_name: "Structure Void".to_string() },
    );

    registry
}
