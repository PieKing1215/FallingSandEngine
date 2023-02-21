use std::fmt::Debug;

use crate::game::common::{registry::Registry, FileHelper};

use self::jigsaw_structure::ConfiguredJigsawFeature;

pub mod jigsaw_structure;

pub trait StructureType {}

pub struct ConfiguredStructurePlaceCtx<'a> {
    pub ecs: &'a mut specs::World,
    pub world_seed: u64,
}

pub trait ConfiguredStructurePlacer: Debug {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceCtx);
}

#[derive(Debug)]
pub struct ConfiguredStructure {
    pub placer: Box<dyn ConfiguredStructurePlacer + Send + Sync>,
}

impl ConfiguredStructurePlacer for ConfiguredStructure {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceCtx) {
        self.placer.place(x, y, ctx);
    }
}

impl ConfiguredStructure {
    pub fn new(placer: impl ConfiguredStructurePlacer + Send + Sync + 'static) -> Self {
        Self { placer: Box::new(placer) }
    }
}

pub type ConfiguredStructureRegistry = Registry<ConfiguredStructure>;

#[allow(clippy::too_many_lines)]
pub fn init_configured_structures(_file_helper: &FileHelper) -> ConfiguredStructureRegistry {
    let mut registry = Registry::new();

    registry.register(
        "test_configured_structure",
        ConfiguredStructure::new(ConfiguredJigsawFeature {
            start_pool: "rooms".into(),
            depth: 8,
            max_distance: 400,
        }),
    );

    registry.register(
        "yellow_thing",
        ConfiguredStructure::new(ConfiguredJigsawFeature {
            start_pool: "yellow_thing".into(),
            depth: 0,
            max_distance: 100,
        }),
    );

    registry
}
