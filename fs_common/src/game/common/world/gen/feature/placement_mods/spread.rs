use std::ops::Range;

use rand::Rng;

use crate::game::common::{
    world::gen::{feature::PlacementModifier, populator::ChunkContext},
    Registries,
};

#[derive(Debug)]
pub struct Spread {
    pub count: u32,
    pub min_dist: f32,
    pub x: Range<i32>,
    pub y: Range<i32>,
}

impl PlacementModifier for Spread {
    fn process(
        &self,
        _chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        _seed: i32,
        rng: &mut dyn rand::RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        let mut v: Vec<(i32, i32)> = vec![];
        for _ in 0..self.count {
            // limited attempts
            for _ in 0..32 {
                let x = pos.0 + rng.gen_range(self.x.clone());
                let y = pos.1 + rng.gen_range(self.y.clone());
                if v.iter().all(|p| {
                    let dx = p.0 - x;
                    let dy = p.1 - y;
                    (dx * dx + dy * dy) as f32 > self.min_dist * self.min_dist
                }) {
                    v.push((x, y));
                    break;
                }
            }
        }
        v
    }
}
