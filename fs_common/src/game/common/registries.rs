use super::{
    world::{
        gen::structure::{
            self, configured_structure::ConfiguredStructureRegistry, pool::StructurePoolRegistry,
            set::StructureSetRegistry,
        },
        material::{
            self,
            placer::{self, MaterialPlacerRegistry},
            MaterialRegistry,
        },
    },
    FileHelper,
};

pub struct Registries {
    pub materials: MaterialRegistry,
    pub material_placers: MaterialPlacerRegistry,
    pub structure_pools: StructurePoolRegistry,
    pub configured_structures: ConfiguredStructureRegistry,
    pub structure_sets: StructureSetRegistry,
}

impl Registries {
    pub fn init(file_helper: &FileHelper) -> Self {
        Self {
            materials: material::init_material_types(),
            material_placers: placer::init_material_placers(file_helper),
            structure_pools: structure::pool::init_structure_pools(file_helper),
            configured_structures: structure::configured_structure::init_configured_structures(
                file_helper,
            ),
            structure_sets: structure::set::init_structure_sets(file_helper),
        }
    }

    pub fn empty() -> Self {
        Self {
            materials: MaterialRegistry::new(),
            material_placers: MaterialPlacerRegistry::new(),
            structure_pools: StructurePoolRegistry::new(),
            configured_structures: ConfiguredStructureRegistry::new(),
            structure_sets: StructureSetRegistry::new(),
        }
    }
}
