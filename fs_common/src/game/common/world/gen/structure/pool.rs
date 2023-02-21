use std::fs;

use serde::Deserialize;

use crate::game::common::{
    registry::{Registry, RegistryID},
    FileHelper,
};

use super::piece::StructurePiece;

#[derive(Debug, Deserialize)]
pub struct StructurePool {
    pub pool: Vec<RegistryID<StructurePiece>>,
}

impl From<Vec<&str>> for StructurePool {
    fn from(value: Vec<&str>) -> Self {
        Self {
            pool: value.into_iter().map(RegistryID::from).collect(),
        }
    }
}

pub type StructurePoolRegistry = Registry<StructurePool>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pools(file_helper: &FileHelper) -> StructurePoolRegistry {
    let mut registry = Registry::new();

    for path in file_helper.files_in_dir_with_ext("data/structure/pool", "ron") {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let bytes = fs::read(path).unwrap();
        let set: StructurePool = ron::de::from_bytes(&bytes).unwrap();

        registry.register(name, set);
    }

    registry
}
