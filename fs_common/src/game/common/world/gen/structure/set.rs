use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
};

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Deserialize;

use crate::game::common::{
    registry::{Registry, RegistryID},
    FileHelper, Registries,
};

use super::configured_structure::ConfiguredStructure;

#[derive(Debug, Deserialize)]
pub struct StructureSet {
    pub structures: Vec<RegistryID<ConfiguredStructure>>,
    pub frequency: f32, // 0.0..=1.0
    pub exclusion: Option<ExclusionZone>,
    /// Average distance between gen attepts
    pub spacing: u16,
    /// Minimum distance between gen attempts
    pub separation: u16,
    pub salt: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExclusionZone {
    pub chunk_distance: u8,
    pub other_set: RegistryID<StructureSet>,
}

pub type StructureSetRegistry = Registry<StructureSet>;

pub fn init_structure_sets(file_helper: &FileHelper) -> StructureSetRegistry {
    let mut registry = Registry::new();

    for path in file_helper.files_in_dir_with_ext("data/structure/set", "ron") {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let bytes = fs::read(path).unwrap();
        let set: StructureSet = ron::de::from_bytes(&bytes).unwrap();

        registry.register(name, set);
    }

    registry
}

impl StructureSet {
    pub fn should_generate_at(
        &self,
        chunk: (i32, i32),
        world_seed: u64,
        registries: &Registries,
        check_exclusion: bool,
    ) -> bool {
        let start = self.nearest_start_chunk(chunk, world_seed);
        if start != chunk {
            return false;
        }

        let mut hasher = DefaultHasher::new();
        chunk.0.hash(&mut hasher);
        chunk.1.hash(&mut hasher);
        self.salt.hash(&mut hasher);
        let hashed = hasher.finish();

        let mut rng = StdRng::seed_from_u64(world_seed.wrapping_add(hashed));

        if rng.gen_range(0.0..1.0) > self.frequency {
            return false;
        }

        if check_exclusion {
            if let Some(exclusion) = &self.exclusion {
                let mut other_can_generate = false;

                // check chunks within range to see if other structure will try to generate there
                let other = registries
                    .structure_sets
                    .get(&exclusion.other_set)
                    .expect(format!("Invalid exclusion set: {:?}", exclusion.other_set).as_str());
                let d = i32::from(exclusion.chunk_distance);
                'outer: for x in (chunk.0 - d)..=(chunk.0 + d) {
                    for y in (chunk.1 - d)..=(chunk.1 + d) {
                        if x == chunk.0 && y == chunk.1 {
                            continue;
                        }
                        if other.should_generate_at((x, y), world_seed, registries, false) {
                            other_can_generate = true;
                            break 'outer;
                        }
                    }
                }

                if other_can_generate {
                    return false;
                }
            }
        }

        true
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
        self.salt.hash(&mut hasher);
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

    pub fn sample_structure(&self) -> &RegistryID<ConfiguredStructure> {
        &self.structures[0]
    }
}
