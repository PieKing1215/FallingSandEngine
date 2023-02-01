pub mod textured;

use std::fs;

use crate::game::common::FileHelper;

use self::textured::TexturedPlacer;

use super::{color::Color, registry::Registry, MaterialID, MaterialInstance, PhysicsType};

pub trait MaterialPlacer: Sync {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance;
}

impl MaterialPlacer for MaterialInstance {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        *self
    }
}

impl<F: Fn() -> MaterialInstance + Sync> MaterialPlacer for F {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        self()
    }
}

#[derive(Debug)]
pub struct MaterialPlacerMeta {
    pub display_name: String,
}

pub type MaterialPlacerID = u16;

pub const AIR_PLACER: MaterialPlacerID = 0;
pub const TEST_PLACER_1: MaterialPlacerID = 1;
pub const TEST_PLACER_2: MaterialPlacerID = 2;

pub const COBBLE_STONE: MaterialPlacerID = 3;
pub const COBBLE_DIRT: MaterialPlacerID = 4;
pub const FADED_COBBLE_STONE: MaterialPlacerID = 5;
pub const FADED_COBBLE_DIRT: MaterialPlacerID = 6;
pub const SMOOTH_STONE: MaterialPlacerID = 7;
pub const SMOOTH_DIRT: MaterialPlacerID = 8;

pub type MaterialPlacerRegistry =
    Registry<MaterialID, (MaterialPlacerMeta, Box<dyn MaterialPlacer + Send>)>;

#[allow(clippy::too_many_lines)]
pub fn init_material_placers(file_helper: &FileHelper) -> MaterialPlacerRegistry {
    let mut registry = Registry::new();

    registry.register(
        AIR_PLACER,
        (
            MaterialPlacerMeta { display_name: "Air".to_string() },
            Box::new(MaterialInstance::air) as Box<dyn MaterialPlacer + Send>,
        ),
    );

    registry.register(
        TEST_PLACER_1,
        (
            MaterialPlacerMeta { display_name: "Test 1".to_string() },
            Box::new(MaterialInstance {
                material_id: super::TEST,
                physics: PhysicsType::Solid,
                color: Color::GRAY,
            }),
        ),
    );

    registry.register(
        TEST_PLACER_2,
        (
            MaterialPlacerMeta { display_name: "Test 2".to_string() },
            Box::new(TexturedPlacer::new(
                super::TEST,
                PhysicsType::Sand,
                &fs::read(file_helper.asset_path("texture/material/test.png")).unwrap(),
            )),
        ),
    );

    registry.register(
        COBBLE_STONE,
        (
            MaterialPlacerMeta { display_name: "Cobblestone".to_string() },
            Box::new(TexturedPlacer::new(
                super::COBBLE_STONE,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/cobble_stone_128x.png"))
                    .unwrap(),
            )),
        ),
    );

    registry.register(
        COBBLE_DIRT,
        (
            MaterialPlacerMeta { display_name: "Cobbledirt".to_string() },
            Box::new(TexturedPlacer::new(
                super::COBBLE_DIRT,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/cobble_dirt_128x.png")).unwrap(),
            )),
        ),
    );

    registry.register(
        FADED_COBBLE_STONE,
        (
            MaterialPlacerMeta { display_name: "Faded Cobblestone".to_string() },
            Box::new(TexturedPlacer::new(
                super::FADED_COBBLE_STONE,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/flat_cobble_stone_128x.png"))
                    .unwrap(),
            )),
        ),
    );

    registry.register(
        FADED_COBBLE_DIRT,
        (
            MaterialPlacerMeta { display_name: "Faded Cobbledirt".to_string() },
            Box::new(TexturedPlacer::new(
                super::FADED_COBBLE_DIRT,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/flat_cobble_dirt_128x.png"))
                    .unwrap(),
            )),
        ),
    );

    registry.register(
        SMOOTH_STONE,
        (
            MaterialPlacerMeta { display_name: "Smooth Stone".to_string() },
            Box::new(TexturedPlacer::new(
                super::SMOOTH_STONE,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/smooth_stone_128x.png"))
                    .unwrap(),
            )),
        ),
    );

    registry.register(
        SMOOTH_DIRT,
        (
            MaterialPlacerMeta { display_name: "Dirt".to_string() },
            Box::new(TexturedPlacer::new(
                super::SMOOTH_DIRT,
                PhysicsType::Solid,
                &fs::read(file_helper.asset_path("texture/material/smooth_dirt_128x.png")).unwrap(),
            )),
        ),
    );

    registry
}
