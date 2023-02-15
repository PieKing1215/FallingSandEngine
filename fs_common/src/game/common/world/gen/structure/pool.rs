use crate::game::common::{
    world::{
        copy_paste::MaterialBuf,
        material::{self, color::Color, registry::Registry, MaterialInstance, PhysicsType},
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

    let structure_a = Arc::new(make_test_structure(
        120,
        120,
        vec![
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
    ));
    let structure_a2 = Arc::new(make_test_structure(
        200,
        100,
        vec![
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
    ));

    let structure_b = Arc::new(make_test_structure(
        100,
        32,
        vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 16, direction_out: Direction::Left },
                StructureNodeConfig::new("rooms").override_depth(),
            ),
            (
                StructureNodeLocalPlacement { x: 100, y: 16, direction_out: Direction::Right },
                StructureNodeConfig::new("rooms").override_depth(),
            ),
        ],
    ));
    let structure_b2 = Arc::new(make_test_structure(
        80,
        80,
        vec![
            (
                StructureNodeLocalPlacement { x: 0, y: 60, direction_out: Direction::Left },
                StructureNodeConfig::new("rooms_or_straight_hallways").override_depth(),
            ),
            (
                StructureNodeLocalPlacement { x: 60, y: 0, direction_out: Direction::Up },
                StructureNodeConfig::new("rooms_or_straight_hallways").override_depth(),
            ),
        ],
    ));

    registry.register("rooms", vec![structure_a.clone(), structure_a2.clone()]);
    registry.register("hallways", vec![structure_b.clone(), structure_b2]);
    registry.register(
        "rooms_or_straight_hallways",
        vec![structure_a, structure_a2, structure_b.clone(), structure_b],
    );

    registry
}

fn make_test_structure(
    w: u16,
    h: u16,
    child_nodes: Vec<(StructureNodeLocalPlacement, StructureNodeConfig)>,
) -> StructureTemplate {
    let mut buf = MaterialBuf::new(w, h, vec![MaterialInstance::air(); (w * h) as usize]).unwrap();

    for x in 0..w {
        for y in 0..h {
            let near_node = child_nodes.iter().any(|cn| {
                let dx = cn.0.x.abs_diff(u32::from(x));
                let dy = cn.0.y.abs_diff(u32::from(y));
                dx < 12 && dy < 12
            });
            if (x < 4 || y < 4 || (x >= w - 4) || (y >= h - 4)) && !near_node {
                buf.set(
                    x,
                    y,
                    MaterialInstance {
                        material_id: material::TEST,
                        physics: PhysicsType::Solid,
                        color: Color::rgb(
                            f32::from(x) / f32::from(w),
                            f32::from(y) / f32::from(h),
                            0.0,
                        ),
                    },
                );
            }
        }
    }

    StructureTemplate { buf, child_nodes }
}
