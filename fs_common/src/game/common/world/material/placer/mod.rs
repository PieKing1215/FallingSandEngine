pub mod lit;
pub mod lit_colored;
pub mod textured;

use std::fs;

use once_cell::sync::Lazy;

use crate::game::common::{
    registry::{Registry, RegistryID},
    FileHelper,
};

use self::{lit_colored::LitColoredExt, textured::TexturedPlacer};

use super::{color::Color, MaterialInstance, PhysicsType};

pub trait MaterialPlacerSampler: Sync {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance;
}

impl MaterialPlacerSampler for MaterialInstance {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        self.clone()
    }
}

impl<F: Fn() -> MaterialInstance + Sync> MaterialPlacerSampler for F {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        self()
    }
}

#[derive(Debug)]
pub struct MaterialPlacerMeta {
    pub display_name: String,
}

pub struct MaterialPlacer {
    pub meta: MaterialPlacerMeta,
    pub sampler: Box<dyn MaterialPlacerSampler + Send>,
}

impl MaterialPlacerSampler for MaterialPlacer {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance {
        self.sampler.pixel(x, y)
    }
}

pub static AIR_PLACER: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "air".into());
pub static TEST_PLACER_1: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "test_placer_1".into());
pub static TEST_PLACER_2: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "test_placer_2".into());
pub static TEST_GRASS: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "test_grass".into());

pub static COBBLE_STONE: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "cobble_stone".into());
pub static COBBLE_DIRT: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "cobble_dirt".into());
pub static FADED_COBBLE_STONE: Lazy<RegistryID<MaterialPlacer>> =
    Lazy::new(|| "faded_cobble_stone".into());
pub static FADED_COBBLE_DIRT: Lazy<RegistryID<MaterialPlacer>> =
    Lazy::new(|| "faded_cobble_dirt".into());
pub static SMOOTH_STONE: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "smooth_stone".into());
pub static SMOOTH_DIRT: Lazy<RegistryID<MaterialPlacer>> = Lazy::new(|| "smooth_dirt".into());

pub type MaterialPlacerRegistry = Registry<MaterialPlacer>;

#[allow(clippy::too_many_lines)]
pub fn init_material_placers(file_helper: &FileHelper) -> MaterialPlacerRegistry {
    let mut registry = Registry::new();

    registry.register(
        AIR_PLACER.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Air".to_string() },
            sampler: Box::new(MaterialInstance::air) as Box<dyn MaterialPlacerSampler + Send>,
        },
    );

    registry.register(
        TEST_PLACER_1.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Test 1".to_string() },
            sampler: Box::new(super::TEST.instance(PhysicsType::Solid, Color::GRAY)),
        },
    );

    registry.register(
        TEST_PLACER_2.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Test 2".to_string() },
            sampler: Box::new(
                TexturedPlacer::new(
                    super::TEST.clone(),
                    PhysicsType::Sand,
                    &fs::read(file_helper.asset_path("texture/material/test.png")).unwrap(),
                )
                .lit_colored(0.5),
            ),
        },
    );

    registry.register(
        TEST_GRASS.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Test Grass".to_string() },
            sampler: Box::new(super::TEST.instance(PhysicsType::Solid, Color::rgb(0, 127, 0))),
        },
    );

    registry.register(
        COBBLE_STONE.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Cobblestone".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::COBBLE_STONE.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/cobble_stone_128x.png"))
                    .unwrap(),
            )),
        },
    );

    registry.register(
        COBBLE_DIRT.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Cobbledirt".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::COBBLE_DIRT.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/cobble_dirt_128x.png")).unwrap(),
            )),
        },
    );

    registry.register(
        FADED_COBBLE_STONE.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Faded Cobblestone".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::FADED_COBBLE_STONE.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/flat_cobble_stone_128x.png"))
                    .unwrap(),
            )),
        },
    );

    registry.register(
        FADED_COBBLE_DIRT.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Faded Cobbledirt".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::FADED_COBBLE_DIRT.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/flat_cobble_dirt_128x.png"))
                    .unwrap(),
            )),
        },
    );

    registry.register(
        SMOOTH_STONE.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Smooth Stone".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::SMOOTH_STONE.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/smooth_stone_128x.png"))
                    .unwrap(),
            )),
        },
    );

    registry.register(
        SMOOTH_DIRT.clone(),
        MaterialPlacer {
            meta: MaterialPlacerMeta { display_name: "Dirt".to_string() },
            sampler: Box::new(TexturedPlacer::new(
                super::SMOOTH_DIRT.clone(),
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/smooth_dirt_128x.png")).unwrap(),
            )),
        },
    );

    // test placers

    let register_test = |color: &str, registry: &mut MaterialPlacerRegistry| {
        registry.register(
            format!("test_{color}"),
            MaterialPlacer {
                meta: MaterialPlacerMeta { display_name: format!("Test {color}") },
                sampler: Box::new(TexturedPlacer::new(
                    super::COBBLE_STONE.clone(),
                    PhysicsType::Solid,
                    &fs::read(file_helper.asset_path(format!("texture/material/test_{color}.png")))
                        .unwrap(),
                )),
            },
        );
    };
    register_test("red", &mut registry);
    register_test("green", &mut registry);
    register_test("blue", &mut registry);
    register_test("magenta", &mut registry);
    register_test("cyan", &mut registry);
    register_test("yellow", &mut registry);
    register_test("white", &mut registry);

    registry
}
