use crate::game::common::{
    registry::{Registry, RegistryID},
    FileHelper,
};

use super::template::StructureTemplate;

pub type StructurePoolID = &'static str;

pub type StructurePoolRegistry = Registry<StructurePoolID, Vec<RegistryID<StructureTemplate>>>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pools(_file_helper: &FileHelper) -> StructurePoolRegistry {
    let mut registry = Registry::new();

    registry.register("rooms", vec!["a".into(), "a2".into()]);
    registry.register("hallways", vec!["b".into(), "b2".into(), "stairs".into()]);
    registry.register(
        "rooms_or_straight_hallways",
        vec!["a".into(), "a2".into(), "b".into(), "b".into()],
    );
    registry.register("end_pieces", vec!["end_carve".into()]);

    registry
}
