use crate::game::common::{world::material::registry::Registry, FileHelper};

use super::template::StructureTemplateID;

pub type StructurePoolID = &'static str;

pub type StructurePoolRegistry = Registry<StructurePoolID, Vec<StructureTemplateID>>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pools(_file_helper: &FileHelper) -> StructurePoolRegistry {
    let mut registry = Registry::new();

    registry.register("rooms", vec!["a", "a2"]);
    registry.register("hallways", vec!["b", "b2", "stairs"]);
    registry.register("rooms_or_straight_hallways", vec!["a", "a2", "b", "b"]);
    registry.register("end_pieces", vec!["end_carve"]);

    registry
}
