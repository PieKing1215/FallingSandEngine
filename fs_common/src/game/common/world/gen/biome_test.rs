use std::sync::Arc;

use crate::game::{
    common::world::material::{self, placer, MaterialInstance, PhysicsType},
    Registries,
};

use rand::Rng;
use simdnoise::NoiseBuilder;

use crate::game::common::world::CHUNK_SIZE;

use super::{
    biome::BiomePlacementParameter,
    feature::{
        features::blob::Blob,
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
            replace_surface_depth: 1,
            searching_for: |m| m.material_id == material::SMOOTH_DIRT,
            replace: |_mat, x, y, registries| {
                Some(
                    registries
                        .material_placers
                        .get(&placer::TEST_PLACER_1)
                        .unwrap()
                        .1
                        .pixel(x, y),
                )
            },
        });

        populators.add(StalactitePopulator {
            searching_for: |m| m.material_id == material::SMOOTH_STONE,
            replace: |mat, x, y, registries| {
                if mat.material_id == material::AIR {
                    Some(
                        registries
                            .material_placers
                            .get(&placer::SMOOTH_STONE)
                            .unwrap()
                            .1
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        populators.add(NearbyReplacePopulator {
            radius: 10,
            searching_for: |m| m.material_id == material::AIR,
            replace: |mat, x, y, registries| {
                if mat.material_id == material::SMOOTH_STONE {
                    Some(
                        registries
                            .material_placers
                            .get(&placer::FADED_COBBLE_STONE)
                            .unwrap()
                            .1
                            .pixel(x, y),
                    )
                } else if mat.material_id == material::SMOOTH_DIRT {
                    Some(
                        registries
                            .material_placers
                            .get(&placer::FADED_COBBLE_DIRT)
                            .unwrap()
                            .1
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        populators.add(NearbyReplacePopulator {
            radius: 6,
            searching_for: |m| m.material_id == material::AIR,
            replace: |mat, x, y, registries| {
                if mat.material_id == material::SMOOTH_STONE
                    || mat.material_id == material::FADED_COBBLE_STONE
                {
                    Some(
                        registries
                            .material_placers
                            .get(&placer::COBBLE_STONE)
                            .unwrap()
                            .1
                            .pixel(x, y),
                    )
                } else if mat.material_id == material::SMOOTH_DIRT
                    || mat.material_id == material::FADED_COBBLE_DIRT
                {
                    Some(
                        registries
                            .material_placers
                            .get(&placer::COBBLE_DIRT)
                            .unwrap()
                            .1
                            .pixel(x, y),
                    )
                } else {
                    None
                }
            },
        });

        let features = vec![
            // PlacedFeature::new(SinglePixel::new(placer::TEST_PLACER_1))
            //     .placement(Count::range(0..=3))
            //     .placement(RandomOffset::chunk())
            //     .placement(Count::range(5..=10))
            //     .placement(RandomOffset::new(-5..6, -5..6))
            //     .placement(MaterialMatch::physics(PhysicsType::Solid)),
            PlacedFeature::new(Blob::new(
                placer::SMOOTH_DIRT,
                Arc::new(|rng| rng.gen_range(16..64)),
                Arc::new(|m| m.physics == PhysicsType::Solid),
                false,
            ))
            .placement(Chance(0.25))
            .placement(Count::range(0..=2))
            .placement(RandomOffset::chunk())
            .placement(MaterialMatch::material(material::SMOOTH_STONE)),
            PlacedFeature::new(Blob::new(
                placer::TEST_PLACER_2,
                Arc::new(|rng| rng.gen_range(10..32)),
                Arc::new(|m| m.physics == PhysicsType::Solid),
                true,
            ))
            .placement(Chance(0.5))
            .placement(Count::range(0..=2))
            .placement(RandomOffset::chunk())
            .placement(MaterialMatch::physics(PhysicsType::Solid)),
        ];

        Self { populators, features }
    }
}

const BIOME_SIZE: u16 = 200;

fn single_random_at(x: f32, y: f32, freq: f32, seed: i32) -> f32 {
    NoiseBuilder::gradient_2d_offset(x, 1, y, 1)
        .with_freq(freq)
        .with_seed(seed)
        .generate()
        .0[0]
}

fn biome_params_at(x: i64, y: i64, seed: i32) -> BiomePlacementParameter {
    let factor_a =
        (single_random_at(x as f32, y as f32, 0.001, seed + 4) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_b =
        (single_random_at(x as f32, y as f32, 0.0005, seed + 5) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_c =
        (single_random_at(x as f32, y as f32, 0.00025, seed + 6) * 20.0 + 0.5).clamp(0.0, 1.0);
    BiomePlacementParameter { a: factor_a, b: factor_b, c: factor_c }
}

#[allow(clippy::cast_lossless)]
fn nearest_biome_point_to(x: i64, y: i64) -> (i64, i64) {
    let bp_x = ((x as f32) / (BIOME_SIZE as f32)).floor() as i64 * (BIOME_SIZE as i64);
    let bp_y = ((y as f32) / (BIOME_SIZE as f32)).floor() as i64 * (BIOME_SIZE as i64);

    (bp_x, bp_y)
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
        let cofs_x = chunk_pixel_x as f32;
        let cofs_y = chunk_pixel_y as f32;

        let (center_biome_point_x, center_biome_point_y) = nearest_biome_point_to(
            (chunk_x as i64 * CHUNK_SIZE as i64) + (CHUNK_SIZE / 2) as i64,
            (chunk_y as i64 * CHUNK_SIZE as i64) + (CHUNK_SIZE / 2) as i64,
        );

        let base_pts = (-2..=2)
            .flat_map(|x| (-2..=2).map(move |y| (x, y)))
            .map(|(x, y)| {
                (
                    center_biome_point_x + x * BIOME_SIZE as i64,
                    center_biome_point_y + y * BIOME_SIZE as i64,
                )
            })
            .collect::<Vec<_>>();

        let vals = base_pts
            .iter()
            .map(|(x, y)| {
                let disp_x = x
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 1)
                        * 20.0
                        * BIOME_SIZE as f32) as i64;
                let disp_y = y
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 2)
                        * 20.0
                        * BIOME_SIZE as f32) as i64;

                let biome = (*super::biome::test::TEST_BIOME_PLACEMENT)
                    .nearest(biome_params_at(*x, *y, seed));

                ((disp_x, disp_y), biome)
            })
            .collect::<Vec<_>>();

        let ofs_x_1 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.005)
                .with_seed(seed + 3)
                .generate()
                .0;

        let ofs_y_1 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.005)
                .with_seed(seed + 4)
                .generate()
                .0;

        let ofs_x_2 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.015)
                .with_seed(seed + 3)
                .generate()
                .0;

        let ofs_y_2 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.015)
                .with_seed(seed + 4)
                .generate()
                .0;

        {
            profiling::scope!("loop");
            for x in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    let i = (x + y * CHUNK_SIZE) as usize;

                    let biome = vals
                        .iter()
                        .min_by(|((x1, y1), _v1), ((x2, y2), _v2)| unsafe {
                            let ox1 = ofs_x_1.get_unchecked(i) * 1000.0;
                            let ox2 = ofs_x_2.get_unchecked(i) * 500.0;
                            let oy1 = ofs_y_1.get_unchecked(i) * 1000.0;
                            let oy2 = ofs_y_2.get_unchecked(i) * 500.0;

                            let ox = x as i64 + cofs_x as i64 + (ox1 + ox2) as i64;
                            let oy = y as i64 + cofs_y as i64 + (oy1 + oy2) as i64;

                            let dx1 = x1 - ox;
                            let dy1 = y1 - oy;
                            let d1 = dx1 * dx1 + dy1 * dy1;

                            let dx2 = x2 - ox;
                            let dy2 = y2 - oy;
                            let d2 = dx2 * dx2 + dy2 * dy2;

                            d1.cmp(&d2)
                        })
                        .unwrap()
                        .1;

                    // using `get_unchecked` has no noticeable performance effect here
                    pixels[i] = biome.pixel(
                        chunk_pixel_x + x as i64,
                        chunk_pixel_y + y as i64,
                        registries,
                    );
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
