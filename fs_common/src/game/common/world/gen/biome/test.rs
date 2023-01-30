use crate::game::{
    common::world::material::{
        self,
        color::Color,
        placer::{self, MaterialPlacerID},
        MaterialInstance, PhysicsType,
    },
    Registries,
};

use super::{Biome, BiomePlacement, BiomePlacementParameter};

lazy_static::lazy_static! {
    pub static ref TEST_BIOME_PLACEMENT: BiomePlacement = BiomePlacement {
        points: vec![
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.0 }, &TestBiome(Color::BLACK)),

            (BiomePlacementParameter { a: 0.75, b: 0.0, c: 0.0 }, &TestBiome(Color::RED)),
            (BiomePlacementParameter { a: 0.0, b: 0.75, c: 0.0 }, &TestBiomePlacer(placer::SMOOTH_DIRT)),
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.75 }, &TestBiome(Color::BLUE)),

            (BiomePlacementParameter { a: 0.5, b: 0.5, c: 0.5 }, &TestBiomePlacer(placer::SMOOTH_STONE)),

            (BiomePlacementParameter { a: 0.25, b: 1.0, c: 1.0 }, &TestBiome(Color::CYAN)),
            (BiomePlacementParameter { a: 1.0, b: 0.25, c: 1.0 }, &TestBiome(Color::MAGENTA)),
            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 0.25 }, &TestBiome(Color::YELLOW)),

            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 1.0 }, &TestBiome(Color::WHITE)),
        ]
    };
}

pub struct TestBiome(Color);

impl Biome for TestBiome {
    fn pixel(&self, _x: i64, _y: i64, _registries: &Registries) -> MaterialInstance {
        MaterialInstance {
            material_id: material::TEST,
            physics: PhysicsType::Object,
            color: self.0,
        }
    }
}

pub struct TestBiomePlacer(MaterialPlacerID);

impl Biome for TestBiomePlacer {
    fn pixel(&self, x: i64, y: i64, registries: &Registries) -> MaterialInstance {
        registries
            .material_placers
            .get(&self.0)
            .unwrap()
            .1
            .pixel(x, y)
    }
}
