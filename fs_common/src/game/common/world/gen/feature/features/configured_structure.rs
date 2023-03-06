use rand::RngCore;

use crate::game::common::{
    registry::RegistryID,
    world::{
        gen::{
            feature::ConfiguredFeature,
            populator::ChunkContext,
            structure::configured_structure::{
                ConfiguredStructure, ConfiguredStructurePlaceContext, ConfiguredStructurePlacer,
            },
        },
        CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct ConfiguredStructureFeature {
    structure: RegistryID<ConfiguredStructure>,
}

impl ConfiguredStructureFeature {
    pub fn new(structure: RegistryID<ConfiguredStructure>) -> Self {
        Self { structure }
    }
}

impl ConfiguredFeature for ConfiguredStructureFeature {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        world_seed: i32,
        _rng: &mut dyn RngCore,
        registries: &Registries,
        ecs: &mut specs::World,
    ) {
        let (cx, cy) = chunks.center_chunk();
        let x = i64::from(cx * i32::from(CHUNK_SIZE)) + i64::from(pos.0);
        let y = i64::from(cy * i32::from(CHUNK_SIZE)) + i64::from(pos.1);

        let configured_structure = registries
            .configured_structures
            .get(&self.structure)
            .unwrap();

        configured_structure.place(
            x,
            y,
            ConfiguredStructurePlaceContext { ecs, world_seed: world_seed as _ },
        );
    }
}
