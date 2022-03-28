use crate::game::common::world::material::{MaterialInstance, TEST_MATERIAL, Color, PhysicsType};

use super::{Biome, BiomePlacement, BiomePlacementParameter};

lazy_static::lazy_static! {
    pub static ref TEST_BIOME_PLACEMENT: BiomePlacement = BiomePlacement {
        points: vec![
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.0 }, &TestBiome(Color::BLACK)),
            
            (BiomePlacementParameter { a: 0.75, b: 0.0, c: 0.0 }, &TestBiome(Color::RED)),
            (BiomePlacementParameter { a: 0.0, b: 0.75, c: 0.0 }, &TestBiome(Color::GREEN)),
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.75 }, &TestBiome(Color::BLUE)),

            (BiomePlacementParameter { a: 0.5, b: 0.5, c: 0.5 }, &TestBiome(Color::GRAY)),

            (BiomePlacementParameter { a: 0.25, b: 1.0, c: 1.0 }, &TestBiome(Color::CYAN)),
            (BiomePlacementParameter { a: 1.0, b: 0.25, c: 1.0 }, &TestBiome(Color::MAGENTA)),
            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 0.25 }, &TestBiome(Color::YELLOW)),

            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 1.0 }, &TestBiome(Color::WHITE)),
        ]
    };
}

pub struct TestBiome(Color);

impl Biome for TestBiome {
    fn pixel(&self) -> MaterialInstance {
        MaterialInstance {
            material_id: TEST_MATERIAL.id,
            physics: PhysicsType::Object,
            color: self.0,
        }
    }
}
