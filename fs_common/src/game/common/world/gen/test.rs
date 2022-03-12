use crate::game::common::world::material::{Color, MaterialInstance, PhysicsType, TEST_MATERIAL};

use simdnoise::NoiseBuilder;

use crate::game::common::world::CHUNK_SIZE;

use super::WorldGenerator;

pub static TEST_GENERATOR: TestGenerator = TestGenerator {};

#[derive(Clone, Copy)]
pub struct TestGenerator {}

impl WorldGenerator for TestGenerator {
    #[allow(clippy::cast_lossless)]
    #[profiling::function]
    fn generate(
        &self,
        chunk_x: i32,
        chunk_y: i32,
        seed: i32,
        pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize],
    ) {
        let cofs_x = (chunk_x * CHUNK_SIZE as i32) as f32;
        let cofs_y = (chunk_y * CHUNK_SIZE as i32) as f32;

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
                    pixels[i] = MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Sand,
                        color: Color::rgb(
                            (f * 191.0) as u8 + 64,
                            64,
                            ((1.0 - f) * 191.0) as u8 + 64,
                        ),
                    };
                    colors[i * 4] = pixels[i].color.r;
                    colors[i * 4 + 1] = pixels[i].color.g;
                    colors[i * 4 + 2] = pixels[i].color.b;
                    colors[i * 4 + 3] = pixels[i].color.a;
                    // chunk.set(x, y, MaterialInstance {
                    //     material_id: TEST_MATERIAL.id,
                    //     physics: crate::game::world::PhysicsType::Solid,
                    //     color: Color::rgb(0, 0, 255),
                    // }).unwrap();
                } else {
                    pixels[i] = MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Solid,
                        color: Color::rgb(80, 64, 32),
                    };
                    colors[i * 4] = pixels[i].color.r;
                    colors[i * 4 + 1] = pixels[i].color.g;
                    colors[i * 4 + 2] = pixels[i].color.b;
                    colors[i * 4 + 3] = pixels[i].color.a;
                    // chunk.set(x, y, MaterialInstance {
                    //     material_id: TEST_MATERIAL.id,
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
}
