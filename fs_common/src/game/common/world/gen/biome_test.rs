use std::sync::Arc;

use crate::game::common::{
    world::material::{
        self,
        placer::{self, MaterialPlacerSampler},
        MaterialInstance, PhysicsType,
    },
    Registries,
};

use rand::Rng;

use crate::game::common::world::CHUNK_SIZE;

use super::{
    feature::{
        features::{blob::Blob, test_structure::TestStructure},
        placement_mods::{
            chance::Chance, count::Count, material_match::MaterialMatch,
            random_offset::RandomOffset,
        },
        PlacedFeature,
    },
    populator::{
        cave::CavePopulator, nearby_replace::NearbyReplacePopulator,
        place_above::PlaceAbovePopulator, spawn::SpawnPopulator, stalactite::StalactitePopulator,
    },
    PopulatorList, WorldGenerator,
};

#[derive(Debug)]
pub struct BiomeTestGenerator {
    populators: PopulatorList,
    features: Vec<PlacedFeature>,
}

impl BiomeTestGenerator {
    #[allow(clippy::new_without_default)]
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        let mut populators = PopulatorList::new();

        populators.add(CavePopulator);
        populators.add(SpawnPopulator);

        populators.add(PlaceAbovePopulator {
            add_surface_height: 1,
            replace_surface_depth: 2,
            searching_for: |m| m.material_id == *material::SMOOTH_DIRT,
            replace: |_mat, x, y, registries| {
                Some(
                    registries
                        .material_placers
                        .get(&*placer::TEST_GRASS)
                        .unwrap()
                        .pixel(x, y),
                )
            },
        });

        populators.add(StalactitePopulator {
            searching_for: |m| m.material_id == *material::SMOOTH_STONE,
            replace: |mat, x, y, registries| {
                if mat.material_id == *material::AIR {
                    Some(
                        registries
                            .material_placers
                            .get(&*placer::SMOOTH_STONE)
                            .unwrap()
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        populators.add(NearbyReplacePopulator {
            radius: 10,
            searching_for: |m| m.material_id == *material::AIR,
            replace: |mat, x, y, registries| {
                if mat.material_id == *material::SMOOTH_STONE {
                    Some(
                        registries
                            .material_placers
                            .get(&*placer::FADED_COBBLE_STONE)
                            .unwrap()
                            .pixel(x, y),
                    )
                } else if mat.material_id == *material::SMOOTH_DIRT {
                    Some(
                        registries
                            .material_placers
                            .get(&*placer::FADED_COBBLE_DIRT)
                            .unwrap()
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        populators.add(NearbyReplacePopulator {
            radius: 6,
            searching_for: |m| m.material_id == *material::AIR,
            replace: |mat, x, y, registries| {
                if mat.material_id == *material::SMOOTH_STONE
                    || mat.material_id == *material::FADED_COBBLE_STONE
                {
                    Some(
                        registries
                            .material_placers
                            .get(&*placer::COBBLE_STONE)
                            .unwrap()
                            .pixel(x, y),
                    )
                } else if mat.material_id == *material::SMOOTH_DIRT
                    || mat.material_id == *material::FADED_COBBLE_DIRT
                {
                    Some(
                        registries
                            .material_placers
                            .get(&*placer::COBBLE_DIRT)
                            .unwrap()
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        let features = vec![
            // PlacedFeature::new(SinglePixel::new(*placer::TEST_PLACER_1))
            //     .placement(Count::range(0..=3))
            //     .placement(RandomOffset::chunk())
            //     .placement(Count::range(5..=10))
            //     .placement(RandomOffset::new(-5..6, -5..6))
            //     .placement(MaterialMatch::physics(PhysicsType::Solid)),
            PlacedFeature::new(Blob::new(
                placer::SMOOTH_DIRT.clone(),
                Arc::new(|rng| rng.gen_range(16..64)),
                Arc::new(|m| m.physics == PhysicsType::Solid),
                false,
            ))
            .placement(Chance(0.25))
            .placement(Count::range(0..=2))
            .placement(RandomOffset::chunk())
            .placement(MaterialMatch::material(material::SMOOTH_STONE.clone())),
            PlacedFeature::new(Blob::new(
                placer::TEST_PLACER_2.clone(),
                Arc::new(|rng| rng.gen_range(10..32)),
                Arc::new(|m| m.physics == PhysicsType::Solid),
                true,
            ))
            .placement(Chance(0.5))
            .placement(Count::range(0..=2))
            .placement(RandomOffset::chunk())
            .placement(MaterialMatch::physics(PhysicsType::Solid)),
            PlacedFeature::new(TestStructure),
        ];

        Self { populators, features }
    }
}

impl WorldGenerator for BiomeTestGenerator {
    #[allow(clippy::cast_lossless)]
    #[profiling::function]
    fn generate(
        &self,
        chunk_x: i32,
        chunk_y: i32,
        seed: i32,
        pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize],
        registries: &Registries,
    ) {
        let chunk_pixel_x = chunk_x as i64 * CHUNK_SIZE as i64;
        let chunk_pixel_y = chunk_y as i64 * CHUNK_SIZE as i64;

        let biomes = registries.biomes.biome_block::<CHUNK_SIZE, CHUNK_SIZE>(
            chunk_pixel_x,
            chunk_pixel_y,
            seed,
        );

        {
            profiling::scope!("loop");
            for x in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let i = (x + y * CHUNK_SIZE) as usize;
                    let biome = biomes[i].1;

                    // using `get_unchecked` has no noticeable performance effect here
                    pixels[i] = biome
                        .base_placer
                        .as_placer(registries)
                        .pixel(chunk_pixel_x + x as i64, chunk_pixel_y + y as i64);
                    colors[i * 4] = pixels[i].color.r;
                    colors[i * 4 + 1] = pixels[i].color.g;
                    colors[i * 4 + 2] = pixels[i].color.b;
                    colors[i * 4 + 3] = pixels[i].color.a;
                }
            }
        }
    }

    fn max_gen_stage(&self) -> u8 {
        2
    }

    fn populators(&self) -> &PopulatorList {
        &self.populators
    }

    fn features(&self) -> &[PlacedFeature] {
        &self.features
    }
}
