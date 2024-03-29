use rand::RngCore;

use crate::game::common::{
    registry::RegistryID,
    world::{
        gen::{feature::ConfiguredFeature, populator::ChunkContext},
        material::placer::MaterialPlacer,
        Chunk, CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct SinglePixel {
    placer_id: RegistryID<MaterialPlacer>,
}

impl SinglePixel {
    pub fn new(placer_id: RegistryID<MaterialPlacer>) -> Self {
        Self { placer_id }
    }
}

impl<C: Chunk> ConfiguredFeature<C> for SinglePixel {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1, C>,
        pos: (i32, i32),
        _seed: i32,
        _rng: &mut dyn RngCore,
        registries: &Registries,
        _ecs: &mut specs::World,
    ) {
        let (cx, cy) = chunks.center_chunk();
        let m = registries
            .material_placers
            .get(&self.placer_id)
            .unwrap()
            .sampler
            .pixel(
                i64::from(cx * i32::from(CHUNK_SIZE)) + i64::from(pos.0),
                i64::from(cy * i32::from(CHUNK_SIZE)) + i64::from(pos.1),
            );
        let _: Result<(), _> = chunks.set(pos.0, pos.1, m);
    }
}
