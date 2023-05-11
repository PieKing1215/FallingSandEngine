use std::fmt::Debug;

use crate::game::common::{registry::Registry, FileHelper};

use self::jigsaw_structure::ConfiguredJigsawFeature;

use super::Direction;

pub mod jigsaw_structure;

pub trait StructureType {}

pub struct ConfiguredStructurePlaceContext<'a> {
    pub ecs: &'a mut specs::World,
    pub world_seed: u64,
}

pub trait ConfiguredStructurePlacer: Debug {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceContext);
}

#[derive(Debug)]
pub struct ConfiguredStructure {
    pub placer: Box<dyn ConfiguredStructurePlacer + Send + Sync>,
}

impl ConfiguredStructurePlacer for ConfiguredStructure {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceContext) {
        self.placer.place(x, y, ctx);
    }
}

impl ConfiguredStructure {
    pub fn new(placer: impl ConfiguredStructurePlacer + Send + Sync + 'static) -> Self {
        Self { placer: Box::new(placer) }
    }
}

pub type ConfiguredStructureRegistry = Registry<ConfiguredStructure>;

pub fn init_configured_structures(_file_helper: &FileHelper) -> ConfiguredStructureRegistry {
    let mut registry = Registry::new();

    registry.register(
        "test_configured_structure",
        ConfiguredStructure::new(ConfiguredJigsawFeature {
            start_pool: "rooms".into(),
            depth: 8,
            max_distance: 400,
            override_dir: None,
        }),
    );

    registry.register(
        "yellow_thing",
        ConfiguredStructure::new(ConfiguredJigsawFeature {
            start_pool: "yellow_thing".into(),
            depth: 0,
            max_distance: 100,
            override_dir: None,
        }),
    );

    registry.register(
        "torch",
        ConfiguredStructure::new(ConfiguredJigsawFeature {
            start_pool: "torch".into(),
            depth: 0,
            max_distance: 100,
            override_dir: Some(Direction::Up),
        }),
    );

    registry
}
