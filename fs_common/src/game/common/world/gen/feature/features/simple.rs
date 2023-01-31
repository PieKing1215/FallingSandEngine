use rand::RngCore;

use crate::game::{
    common::world::{
        gen::{feature::ConfiguredFeature, populator::ChunkContext},
        material::placer::MaterialPlacerID,
        CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct SinglePixel {
    placer_id: MaterialPlacerID,
}

impl SinglePixel {
    pub fn new(placer_id: MaterialPlacerID) -> Self {
        Self { placer_id }
    }
}

impl ConfiguredFeature for SinglePixel {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        _seed: i32,
        _rng: &mut dyn RngCore,
        registries: &Registries,
    ) {
        let (cx, cy) = chunks.center_chunk();
        let m = registries
            .material_placers
            .get(&self.placer_id)
            .unwrap()
            .1
            .pixel(
                i64::from(cx * i32::from(CHUNK_SIZE)) + i64::from(pos.0),
                i64::from(cy * i32::from(CHUNK_SIZE)) + i64::from(pos.1),
            );
        let _ = chunks.set(pos.0, pos.1, m);
    }
}
