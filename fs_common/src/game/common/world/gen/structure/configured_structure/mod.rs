use crate::game::common::{world::material::registry::Registry, FileHelper};

use self::jigsaw_structure::ConfiguredJigsawFeature;

pub mod jigsaw_structure;

pub trait StructureType {}

pub struct ConfiguredStructurePlaceCtx<'a> {
    pub ecs: &'a mut specs::World,
    pub world_seed: u64,
}

pub trait ConfiguredStructure {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceCtx);
}

pub type ConfiguredStructureID = &'static str;

pub type ConfiguredStructureRegistry =
    Registry<ConfiguredStructureID, Box<dyn ConfiguredStructure + Send + Sync>>;

#[allow(clippy::too_many_lines)]
pub fn init_configured_structures(_file_helper: &FileHelper) -> ConfiguredStructureRegistry {
    let mut registry = Registry::new();

    registry.register(
        "test_configured_structure",
        Box::new(ConfiguredJigsawFeature { start_pool: "rooms", depth: 3, max_distance: 200 }) as _,
    );

    registry
}
