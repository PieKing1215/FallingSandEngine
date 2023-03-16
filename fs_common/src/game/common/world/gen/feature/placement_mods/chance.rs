use rand::Rng;

use crate::game::common::{
    world::{
        gen::{feature::PlacementModifier, populator::ChunkContext},
        Chunk,
    },
    Registries,
};

#[derive(Debug)]
pub struct Chance(pub f32);

impl<C: Chunk> PlacementModifier<C> for Chance {
    fn process(
        &self,
        _chunks: &mut ChunkContext<1, C>,
        pos: (i32, i32),
        _seed: i32,
        rng: &mut dyn rand::RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        if rng.gen_range(0.0..1.0) < self.0 {
            vec![pos]
        } else {
            vec![]
        }
    }
}
