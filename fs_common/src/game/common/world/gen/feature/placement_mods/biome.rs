use std::sync::Arc;

use crate::game::common::{
    registry::RegistryID,
    world::{
        gen::{biome::Biome, feature::PlacementModifier, populator::ChunkContext},
        CHUNK_SIZE,
    },
    Registries,
};

pub type BiomeMatchFn = dyn Fn(&RegistryID<Biome>, &Biome) -> bool + Send + Sync;

pub struct BiomeMatch {
    predicate: Arc<BiomeMatchFn>,
}

impl BiomeMatch {
    pub fn new(predicate: Arc<BiomeMatchFn>) -> Self {
        Self { predicate }
    }

    pub fn only(id: impl Into<RegistryID<Biome>>) -> Self {
        let id = id.into();
        Self::new(Arc::new(move |found_id, _| *found_id == id))
    }
}

impl std::fmt::Debug for BiomeMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BiomeMatch").finish()
    }
}

impl PlacementModifier for BiomeMatch {
    fn process(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        seed: i32,
        _rng: &mut dyn rand::RngCore,
        registries: &Registries,
    ) -> Vec<(i32, i32)> {
        let world_x = i64::from(chunks.center_chunk().0) * i64::from(CHUNK_SIZE) + i64::from(pos.0);
        let world_y = i64::from(chunks.center_chunk().1) * i64::from(CHUNK_SIZE) + i64::from(pos.1);

        let (biome_id, biome) = registries.biomes.biome_at(world_x, world_y, seed);

        if (self.predicate)(biome_id, biome) {
            vec![pos]
        } else {
            vec![]
        }
    }
}
