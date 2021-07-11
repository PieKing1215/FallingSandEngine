
use material::TEST_MATERIAL;

use sdl2::pixels::Color;
use simdnoise::NoiseBuilder;

use crate::game::world::{CHUNK_SIZE, MaterialInstance, material};

use super::WorldGenerator;


pub static TEST_GENERATOR: TestGenerator = TestGenerator{};

#[derive(Clone, Copy)]
pub struct TestGenerator {

}

impl WorldGenerator for TestGenerator {
    fn generate(&self, chunk_x: i32, chunk_y: i32, seed: i32, pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize], colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]) {

        let cofs_x = (chunk_x * CHUNK_SIZE as i32) as f32;
        let cofs_y = (chunk_y * CHUNK_SIZE as i32) as f32;

        let noise_cave_2 = NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
            .with_freq(0.002)
            .with_seed(seed)
            .generate().0;

        let noise2_r = NoiseBuilder::gradient_2d_offset(cofs_x + 1238.651, CHUNK_SIZE.into(), cofs_y + 1378.529, CHUNK_SIZE.into())
            .with_freq(0.004)
            .with_seed(seed)
            .generate();
        let noise2 = noise2_r.0;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let i = (x + y * CHUNK_SIZE) as usize;
                let v = noise_cave_2[i];
                let v2 = noise2[i];
                if v > 0.0 {
                    pixels[i] = MaterialInstance::air();
                    // chunk.set(x, y, MaterialInstance::air()).unwrap();
                } else{
                    if v2 > 0.0 {
                        pixels[i] = MaterialInstance {
                            material_id: TEST_MATERIAL.id,
                            physics: crate::game::world::PhysicsType::Sand,
                            color: Color::RGB(0, 0, 255),
                        };
                        colors[i*4 + 0] = 0;
                        colors[i*4 + 1] = 255;
                        colors[i*4 + 2] = 0;
                        colors[i*4 + 3] = 255;
                        // chunk.set(x, y, MaterialInstance {
                        //     material_id: TEST_MATERIAL.id,
                        //     physics: crate::game::world::PhysicsType::Solid,
                        //     color: Color::RGB(0, 0, 255),
                        // }).unwrap();
                    }else{
                        pixels[i] = MaterialInstance {
                            material_id: TEST_MATERIAL.id,
                            physics: crate::game::world::PhysicsType::Solid,
                            color: Color::RGB(0, 255, 0),
                        };
                        colors[i*4 + 0] = 255;
                        colors[i*4 + 1] = 0;
                        colors[i*4 + 2] = 0;
                        colors[i*4 + 3] = 255;
                        // chunk.set(x, y, MaterialInstance {
                        //     material_id: TEST_MATERIAL.id,
                        //     physics: crate::game::world::PhysicsType::Solid,
                        //     color: Color::RGB(0, 255, 0),
                        // }).unwrap();
                    }
                }
            }
        }
    }

    fn max_gen_stage(&self) -> u8 {
        2
    }
}