use crate::game::common::{
    world::{
        material::{self, color::Color, MaterialInstance, PhysicsType},
        Chunk, CHUNK_AREA,
    },
    Registries,
};

use chunksystem::ChunkKey;
use simdnoise::NoiseBuilder;

use crate::game::common::world::CHUNK_SIZE;

use super::{feature::PlacedFeature, PopulatorList, WorldGenerator};

#[derive(Debug)]
pub struct TestGenerator<C: Chunk> {
    populators: PopulatorList<C>,
}

impl<C: Chunk + 'static> TestGenerator<C> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { populators: PopulatorList::new() }
    }
}

impl<C: Chunk + Send + Sync> WorldGenerator<C> for TestGenerator<C> {
    #[allow(clippy::cast_lossless)]
    #[profiling::function]
    fn generate(
        &self,
        chunk_pos: ChunkKey,
        seed: i32,
        pixels: &mut [MaterialInstance; CHUNK_AREA],
        colors: &mut [Color; CHUNK_AREA],
        _background: &mut [MaterialInstance; CHUNK_AREA],
        _background_colors: &mut [Color; CHUNK_AREA],
        _registries: &Registries,
    ) {
        let cofs_x = (chunk_pos.0 * CHUNK_SIZE as i32) as f32;
        let cofs_y = (chunk_pos.1 * CHUNK_SIZE as i32) as f32;

        let noise_cave_2 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.002)
                .with_seed(seed)
                .generate()
                .0;

        let noise2_r = NoiseBuilder::gradient_2d_offset(
            cofs_x + 1238.651,
            CHUNK_SIZE.into(),
            cofs_y + 1378.529,
            CHUNK_SIZE.into(),
        )
        .with_freq(0.004)
        .with_seed(seed)
        .generate();
        let noise2 = noise2_r.0;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let i = (x + y * CHUNK_SIZE) as usize;
                let v = noise_cave_2[i];
                let v2 = noise2[i];
                if v > 0.0
                    || ((32..64).contains(&x)
                        && (32..64).contains(&y)
                        && !((40..56).contains(&x)
                            && (40..56).contains(&y)
                            && !(47..49).contains(&x)))
                {
                    pixels[i] = MaterialInstance::air();
                    // chunk.set(x, y, MaterialInstance::air()).unwrap();
                } else if v2 > 0.0 {
                    let f = (v2 / 0.02).clamp(0.0, 1.0);
                    pixels[i] = material::TEST.instance(
                        PhysicsType::Sand,
                        Color::rgb((f * 191.0) as u8 + 64, 64, ((1.0 - f) * 191.0) as u8 + 64),
                    );
                    colors[i] = pixels[i].color;
                    // chunk.set(x, y, MaterialInstance {
                    //     material_id: material::TEST,
                    //     physics: crate::game::world::PhysicsType::Solid,
                    //     color: Color::rgb(0, 0, 255),
                    // }).unwrap();
                } else {
                    pixels[i] = material::TEST.instance(PhysicsType::Solid, Color::rgb(80, 64, 32));
                    colors[i] = pixels[i].color;
                    // chunk.set(x, y, MaterialInstance {
                    //     material_id: material::TEST,
                    //     physics: crate::game::world::PhysicsType::Solid,
                    //     color: Color::rgb(0, 255, 0),
                    // }).unwrap();
                }
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
        &[]
    }
}
