use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::game::common::{world::material::registry::Registry, FileHelper};

use super::configured_structure::ConfiguredStructureID;

pub struct StructureSet {
    pub structures: Vec<ConfiguredStructureID>,
    pub frequency: f32, // 0.0..=1.0
    pub exclusion: Option<ExclusionZone>,
    /// Average distance between gen attepts
    pub spacing: u16,
    /// Minimum distance between gen attempts
    pub separation: u16,
}

pub struct ExclusionZone {
    pub chunk_distance: u8,
    pub other_set: StructureSetID,
}

pub type StructureSetID = &'static str;

pub type StructureSetRegistry = Registry<StructureSetID, StructureSet>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_sets(_file_helper: &FileHelper) -> StructureSetRegistry {
    let mut registry = Registry::new();

    registry.register(
        "test_structure_set",
        StructureSet {
            structures: vec!["test_configured_structure"],
            frequency: 1.0,
            exclusion: None,
            spacing: 16,
            separation: 5,
        },
    );

    registry
}

impl StructureSet {
    pub fn should_generate_at(&self, chunk: (i32, i32), world_seed: u64) -> bool {
        let start = self.nearest_start_chunk(chunk, world_seed);
        // TODO: implement frequency & exclusion
        start == chunk
    }

    pub fn nearest_start_chunk(&self, chunk: (i32, i32), world_seed: u64) -> (i32, i32) {
        let spacing = i32::from(self.spacing);
        let range = self.spacing - self.separation;

        // div_euclid is equivalent to div_floor here (div_floor isn't stable)
        let spacing_x = chunk.0.div_euclid(spacing);
        let spacing_y = chunk.1.div_euclid(spacing);

        let mut hasher = DefaultHasher::new();
        spacing_x.hash(&mut hasher);
        spacing_y.hash(&mut hasher);
        let hashed = hasher.finish();

        let mut rng = StdRng::seed_from_u64(world_seed.wrapping_add(hashed));
        let ofs_x = if range == 0 {
            0
        } else {
            i32::from(rng.gen_range(0..range))
        };
        let ofs_y = if range == 0 {
            0
        } else {
            i32::from(rng.gen_range(0..range))
        };

        // let (ofs_x, ofs_y) = (0, 0);
        // let (ofs_x, ofs_y) = ((range - 1) as i32, (range - 1) as i32);

        (spacing_x * spacing + ofs_x, spacing_y * spacing + ofs_y)
    }

    pub fn sample_structure(&self) -> ConfiguredStructureID {
        self.structures[0]
    }
}
