use std::ops::Range;

use rand::{Rng, RngCore};

use crate::game::{
    common::world::{
        gen::{feature::PlacementModifier, populator::ChunkContext},
        CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct RandomOffset {
    x: Range<i32>,
    y: Range<i32>,
}

impl RandomOffset {
    pub fn new(x: Range<i32>, y: Range<i32>) -> Self {
        Self { x, y }
    }

    pub fn chunk() -> Self {
        Self {
            x: 0..i32::from(CHUNK_SIZE),
            y: 0..i32::from(CHUNK_SIZE),
        }
    }
}

impl PlacementModifier for RandomOffset {
    fn process(
        &self,
        _chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        _seed: i32,
        rng: &mut dyn RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        let x = pos.0 + rng.gen_range(self.x.clone());
        let y = pos.1 + rng.gen_range(self.y.clone());
        vec![(x, y)]
    }
}
