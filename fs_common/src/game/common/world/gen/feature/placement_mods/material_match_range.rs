use std::ops::Range;

use crate::game::common::{
    world::gen::{feature::PlacementModifier, populator::ChunkContext},
    Registries,
};

use super::material_match::MaterialMatch;

pub struct MaterialMatchRange {
    pub matcher: MaterialMatch,
    pub x: Range<i32>,
    pub y: Range<i32>,
}

impl std::fmt::Debug for MaterialMatchRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialMatchRange").finish()
    }
}

impl PlacementModifier for MaterialMatchRange {
    fn process(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        seed: i32,
        rng: &mut dyn rand::RngCore,
        registries: &Registries,
    ) -> Vec<(i32, i32)> {
        for dx in self.x.clone() {
            for dy in self.y.clone() {
                if self
                    .matcher
                    .process(chunks, (pos.0 + dx, pos.1 + dy), seed, rng, registries)
                    .is_empty()
                {
                    return vec![];
                }
            }
        }

        vec![pos]
    }
}
