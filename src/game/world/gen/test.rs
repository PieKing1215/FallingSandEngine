
use material::TEST_MATERIAL;
use rand::Rng;
use sdl2::pixels::Color;
use simdnoise::NoiseBuilder;

use crate::game::world::{CHUNK_SIZE, MaterialInstance, material};

use super::WorldGenerator;


pub struct TestGenerator {

}

impl WorldGenerator for TestGenerator {
    fn generate(&self, chunk: &mut crate::game::world::Chunk) {

        let cofs_x = (chunk.chunk_x * CHUNK_SIZE as i32) as f32;
        let cofs_y = (chunk.chunk_y * CHUNK_SIZE as i32) as f32;

        let noise_cave_2 = NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
            .with_freq(0.002)
            .generate().0;

        let noise2_r = NoiseBuilder::gradient_2d_offset(cofs_x + 1238.651, CHUNK_SIZE.into(), cofs_y + 1378.529, CHUNK_SIZE.into())
            .with_freq(0.004)
            .generate();
        let noise2 = noise2_r.0;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let i = (x + y * CHUNK_SIZE) as usize;
                let v = noise_cave_2[i];
                let v2 = noise2[i];
                if v > 0.0 {
                    chunk.set(x, y, MaterialInstance::air()).unwrap();
                } else{
                    if v2 > 0.0 {
                        chunk.set(x, y, MaterialInstance {
                            material_id: TEST_MATERIAL.id,
                            physics: crate::game::world::PhysicsType::Solid,
                            color: Color::RGB(0, 0, 255),
                        }).unwrap();
                    }else{
                        chunk.set(x, y, MaterialInstance {
                            material_id: TEST_MATERIAL.id,
                            physics: crate::game::world::PhysicsType::Solid,
                            color: Color::RGB(0, 255, 0),
                        }).unwrap();
                    }
                }
            }
        }
    }
}