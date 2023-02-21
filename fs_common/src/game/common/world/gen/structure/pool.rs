use crate::game::common::{
    registry::{Registry, RegistryID},
    FileHelper,
};

use super::template::StructureTemplate;

#[derive(Debug)]
pub struct StructurePool {
    pub pool: Vec<RegistryID<StructureTemplate>>,
}

impl From<Vec<&str>> for StructurePool {
    fn from(value: Vec<&str>) -> Self {
        Self {
            pool: value.into_iter().map(RegistryID::from).collect(),
        }
    }
}

pub type StructurePoolRegistry = Registry<RegistryID<StructurePool>, StructurePool>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pools(_file_helper: &FileHelper) -> StructurePoolRegistry {
    let mut registry = Registry::new();

    registry.register("rooms", vec!["a", "a2"].into());
    registry.register("hallways", vec!["b", "b2", "stairs"].into());
    registry.register(
        "rooms_or_straight_hallways",
        vec!["a", "a2", "b", "b"].into(),
    );
    registry.register("end_pieces", vec!["end_carve"].into());

    registry
}
