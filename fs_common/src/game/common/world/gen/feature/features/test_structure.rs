use crate::game::common::{
    world::{
        gen::{
            feature::ConfiguredFeature,
            populator::ChunkContext,
            structure::configured_structure::{
                ConfiguredStructurePlaceContext, ConfiguredStructurePlacer,
            },
        },
        Chunk, CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct TestStructure;

impl<C: Chunk> ConfiguredFeature<C> for TestStructure {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1, C>,
        pos: (i32, i32),
        world_seed: i32,
        _rng: &mut dyn rand::RngCore,
        registries: &Registries,
        ecs: &mut specs::World,
    ) {
        let (cx, cy) = chunks.center_chunk();
        let x = i64::from(cx * i32::from(CHUNK_SIZE)) + i64::from(pos.0);
        let y = i64::from(cy * i32::from(CHUNK_SIZE)) + i64::from(pos.1);

        for (_, v) in &registries.structure_sets {
            if v.should_generate_at((cx, cy), world_seed as _, registries, true) {
                let configured_structure = registries
                    .configured_structures
                    .get(v.sample_structure())
                    .unwrap();

                configured_structure.place(
                    x,
                    y,
                    ConfiguredStructurePlaceContext { ecs, world_seed: world_seed as _ },
                );
            }
        }
    }
}
