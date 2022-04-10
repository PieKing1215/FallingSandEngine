pub mod textured;

use std::fs;

use crate::game::common::FileHelper;

use self::textured::TexturedPlacer;

use super::{color::Color, registry::Registry, MaterialID, MaterialInstance, PhysicsType};

pub trait MaterialPlacer {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance;
}

impl MaterialPlacer for MaterialInstance {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        *self
    }
}

impl<F: Fn() -> MaterialInstance> MaterialPlacer for F {
    fn pixel(&self, _x: i64, _y: i64) -> MaterialInstance {
        self()
    }
}

#[derive(Debug)]
pub struct MaterialPlacerMeta {
    pub display_name: String,
}

pub type MaterialPlacerID = u16;

pub static AIR_PLACER: MaterialPlacerID = 0;
pub static TEST_PLACER_1: MaterialPlacerID = 1;
pub static TEST_PLACER_2: MaterialPlacerID = 2;

pub type MaterialPlacerRegistry =
    Registry<MaterialID, (MaterialPlacerMeta, Box<dyn MaterialPlacer>)>;

pub fn init_material_placers(file_helper: &FileHelper) -> MaterialPlacerRegistry {
    let mut registry = Registry::new();

    registry.register(
        AIR_PLACER,
        (
            MaterialPlacerMeta { display_name: "Air".to_string() },
            Box::new(MaterialInstance::air) as Box<dyn MaterialPlacer>,
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

    registry
}
