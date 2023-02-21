use std::fmt::Debug;

use crate::game::common::{registry::Registry, FileHelper};

use self::jigsaw_structure::ConfiguredJigsawFeature;

pub mod jigsaw_structure;

pub trait StructureType {}

pub struct ConfiguredStructurePlaceCtx<'a> {
    pub ecs: &'a mut specs::World,
    pub world_seed: u64,
}

pub trait ConfiguredStructure: Debug {
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
        Box::new(ConfiguredJigsawFeature {
            start_pool: "rooms".into(),
            depth: 8,
            max_distance: 400,
        }) as _,
    );

    registry
}
