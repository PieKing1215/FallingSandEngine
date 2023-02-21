use crate::game::common::{
    registry::RegistryID,
    world::material::{
        self,
        color::Color,
        placer::{self, MaterialPlacer, MaterialPlacerSampler},
        MaterialInstance, PhysicsType,
    },
    Registries,
};

use super::{Biome, BiomePlacement, BiomePlacementParameter};

lazy_static::lazy_static! {
    pub static ref TEST_BIOME_PLACEMENT: BiomePlacement = BiomePlacement {
        points: vec![
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.0 }, Box::new(TestBiomePlacer(placer::SMOOTH_DIRT.clone()))),

            (BiomePlacementParameter { a: 0.75, b: 0.0, c: 0.0 }, Box::new(TestBiome(Color::RED))),
            (BiomePlacementParameter { a: 0.0, b: 0.75, c: 0.0 }, Box::new(TestBiome(Color::GREEN))),
            (BiomePlacementParameter { a: 0.0, b: 0.0, c: 0.75 }, Box::new(TestBiome(Color::BLUE))),

            (BiomePlacementParameter { a: 0.5, b: 0.5, c: 0.5 }, Box::new(TestBiomePlacer(placer::SMOOTH_STONE.clone()))),

            (BiomePlacementParameter { a: 0.25, b: 1.0, c: 1.0 }, Box::new(TestBiome(Color::CYAN))),
            (BiomePlacementParameter { a: 1.0, b: 0.25, c: 1.0 }, Box::new(TestBiome(Color::MAGENTA))),
            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 0.25 }, Box::new(TestBiome(Color::YELLOW))),

            (BiomePlacementParameter { a: 1.0, b: 1.0, c: 1.0 }, Box::new(TestBiome(Color::WHITE))),
        ]
    };
}

pub struct TestBiome(Color);

impl Biome for TestBiome {
    fn pixel(&self, _x: i64, _y: i64, _registries: &Registries) -> MaterialInstance {
        MaterialInstance {
            material_id: material::TEST.clone(),
            physics: PhysicsType::Solid,
            color: self.0,
        }
    }
}

pub struct TestBiomePlacer(RegistryID<MaterialPlacer>);

impl Biome for TestBiomePlacer {
    fn pixel(&self, x: i64, y: i64, registries: &Registries) -> MaterialInstance {
        registries
            .material_placers
            .get(&self.0)
            .unwrap()
            .pixel(x, y)
    }
}
