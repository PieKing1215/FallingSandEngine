use crate::game::common::{
    world::{
        copy_paste::MaterialBuf,
        material::{registry::Registry, MaterialInstance},
    },
    FileHelper,
};
use std::sync::Arc;

use super::{
    template::{StructureNodeConfig, StructureNodeLocalPlacement, StructureTemplate},
    Direction,
};

pub type StructurePoolID = &'static str;

pub type StructurePoolRegistry = Registry<StructurePoolID, Vec<Arc<StructureTemplate>>>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pools(_file_helper: &FileHelper) -> StructurePoolRegistry {
    let mut registry = Registry::new();

    let structure_a = Arc::new(StructureTemplate {
        buf: MaterialBuf::new(
            120,
            120,
            vec![MaterialInstance::air(); (120 * 120) as usize],
        )
        .unwrap(),
        child_nodes: vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 60, direction_out: Direction::Left },
                StructureNodeConfig::new("hallways"),
            ),
            (
                StructureNodeLocalPlacement { x: 120, y: 40, direction_out: Direction::Right },
                StructureNodeConfig::new("hallways"),
            ),
            (
                StructureNodeLocalPlacement { x: 120, y: 80, direction_out: Direction::Right },
                StructureNodeConfig::new("hallways"),
            ),
            (
                StructureNodeLocalPlacement { x: 40, y: 0, direction_out: Direction::Up },
                StructureNodeConfig::new("hallways"),
            ),
            (
                StructureNodeLocalPlacement { x: 80, y: 120, direction_out: Direction::Down },
                StructureNodeConfig::new("hallways"),
            ),
        ],
    });
    let structure_a2 = Arc::new(StructureTemplate {
        buf: MaterialBuf::new(
            200,
            100,
            vec![MaterialInstance::air(); (200 * 100) as usize],
        )
        .unwrap(),
        child_nodes: vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 50, direction_out: Direction::Left },
                StructureNodeConfig::new("hallways")
                    .block_in_dirs(vec![Direction::Up, Direction::Down]),
            ),
            (
                StructureNodeLocalPlacement { x: 200, y: 20, direction_out: Direction::Right },
                StructureNodeConfig::new("hallways")
                    .block_in_dirs(vec![Direction::Up, Direction::Down]),
            ),
        ],
    });

    let structure_b = Arc::new(StructureTemplate {
        buf: MaterialBuf::new(100, 25, vec![MaterialInstance::air(); (100 * 25) as usize]).unwrap(),
        child_nodes: vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 12, direction_out: Direction::Left },
                StructureNodeConfig::new("rooms").override_depth(),
            ),
            (
                StructureNodeLocalPlacement { x: 100, y: 12, direction_out: Direction::Right },
                StructureNodeConfig::new("rooms").override_depth(),
            ),
        ],
    });
    let structure_b2 = Arc::new(StructureTemplate {
        buf: MaterialBuf::new(80, 80, vec![MaterialInstance::air(); (80 * 80) as usize]).unwrap(),
        child_nodes: vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 60, direction_out: Direction::Left },
                StructureNodeConfig::new("rooms_or_straight_hallways").override_depth(),
            ),
            (
                StructureNodeLocalPlacement { x: 60, y: 0, direction_out: Direction::Up },
                StructureNodeConfig::new("rooms_or_straight_hallways").override_depth(),
            ),
        ],
    });

    registry.register("rooms", vec![structure_a.clone(), structure_a2.clone()]);
    registry.register("hallways", vec![structure_b.clone(), structure_b2]);
    registry.register(
        "rooms_or_straight_hallways",
        vec![structure_a, structure_a2, structure_b.clone(), structure_b],
    );

    registry
}
