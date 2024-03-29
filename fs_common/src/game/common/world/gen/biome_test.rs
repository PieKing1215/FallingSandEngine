use std::sync::Arc;

use crate::game::common::world::{
    chunk_index::{ChunkLocalIndex, ChunkLocalPosition},
    material::{
        self,
        placer::{self, MaterialPlacerSampler},
        PhysicsType,
    },
    Chunk, CHUNK_AREA,
};

use chunksystem::ChunkKey;
use rand::Rng;

use crate::game::common::world::CHUNK_SIZE;

use super::{
    feature::{
        features::{
            blob::Blob, configured_structure::ConfiguredStructureFeature,
            test_structure::TestStructure,
        },
        placement_mods::{
            biome::BiomeMatch, chance::Chance, count::Count, material_match::MaterialMatch,
            material_match_range::MaterialMatchRange, on_ground::OnGround,
            random_offset::RandomOffset, spread::Spread,
        },
        PlacedFeature,
    },
    populator::{
        cave::CavePopulator, nearby_replace::NearbyReplacePopulator,
        place_above::PlaceAbovePopulator, spawn::SpawnPopulator, stalactite::StalactitePopulator,
    },
    GenBuffers, GenContext, PopulatorList, WorldGenerator,
};

#[derive(Debug)]
pub struct BiomeTestGenerator<C: Chunk> {
    populators: PopulatorList<C>,
    features: Vec<PlacedFeature<C>>,
}

impl<C: Chunk + 'static> BiomeTestGenerator<C> {
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
            // PlacedFeature::new(SinglePixel::new(placer::TEST_PLACER_2.clone()))
            //     .placement(Count::range(30..=40))
            //     .placement(RandomOffset::chunk())
            //     .placement(MaterialMatch::physics(PhysicsType::Solid))
            //     .placement(BiomeMatch::only("main")),
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
            .placement(MaterialMatch::physics(PhysicsType::Solid))
            .placement(BiomeMatch::only("main")),
            PlacedFeature::new(ConfiguredStructureFeature::new("yellow_thing".into()))
                .placement(Count::range(0..=2))
                .placement(RandomOffset::chunk())
                .placement(MaterialMatch::physics(PhysicsType::Solid))
                .placement(BiomeMatch::only("yellow")),
            PlacedFeature::new(ConfiguredStructureFeature::new("torch".into()))
                .placement(Chance(0.5))
                .placement(Spread {
                    count: 3,
                    min_dist: 10.0,
                    x: 2..i32::from(CHUNK_SIZE) - 2,
                    y: 0..1,
                })
                .placement(RandomOffset::chunk_y())
                .placement(OnGround { max_distance: Some(u32::from(CHUNK_SIZE / 2)) })
                .placement(MaterialMatchRange {
                    matcher: MaterialMatch::physics(PhysicsType::Air),
                    x: 0..1,
                    y: -10..0,
                }),
            PlacedFeature::new(TestStructure),
        ];

        Self { populators, features }
    }
}

impl<C: Chunk + 'static> Default for BiomeTestGenerator<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Chunk + Send + Sync> WorldGenerator<C> for BiomeTestGenerator<C> {
    #[allow(clippy::cast_lossless)]
    #[profiling::function]
    fn generate(&self, chunk_pos: ChunkKey, mut buf: GenBuffers, ctx: GenContext) {
        let chunk_pixel_x = chunk_pos.0 as i64 * CHUNK_SIZE as i64;
        let chunk_pixel_y = chunk_pos.1 as i64 * CHUNK_SIZE as i64;

        // `biome_block` always returns Vec with size W*H, but this cannot be expressed until `generic_const_exprs` is stable
        let Ok(biomes): Result<[_; CHUNK_AREA], _> = ctx.registries.biomes.biome_block::<CHUNK_SIZE, CHUNK_SIZE>(
            chunk_pixel_x,
            chunk_pixel_y,
            ctx.seed,
        ).try_into() else { unreachable!() };

        {
            profiling::scope!("loop");
            for p in ChunkLocalPosition::iter() {
                let i: ChunkLocalIndex = p.into();
                let biome = biomes[i].1;

                buf.set_pixel(
                    i,
                    biome
                        .base_placer
                        .as_placer(ctx.registries)
                        .pixel(chunk_pixel_x + p.x() as i64, chunk_pixel_y + p.y() as i64),
                );

                buf.set_bg(
                    i,
                    biome
                        .base_placer
                        .as_placer(ctx.registries)
                        .pixel(chunk_pixel_x + p.x() as i64, chunk_pixel_y + p.y() as i64),
                );
            }
        }
    }

    fn max_gen_stage(&self) -> u8 {
        2
    }

    fn populators(&self) -> &PopulatorList<C> {
        &self.populators
    }

    fn features(&self) -> &[PlacedFeature<C>] {
        &self.features
    }
}
