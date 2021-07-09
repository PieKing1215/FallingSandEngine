
use rand::Rng;
use sdl2::pixels::Color;
use simdnoise::NoiseBuilder;

use crate::game::world::CHUNK_SIZE;

use super::WorldGenerator;


pub struct TestGenerator {

}

impl WorldGenerator for TestGenerator {
    fn generate(&self, chunk: &mut crate::game::world::Chunk) {

        let cofs_x = (chunk.chunk_x * CHUNK_SIZE as i32) as f32;
        let cofs_y = (chunk.chunk_y * CHUNK_SIZE as i32) as f32;

        let noise = NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
            .with_freq(0.005)
            .generate_scaled(0.0, 1.0);

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let i = x + y * CHUNK_SIZE;
                let v = noise[i as usize];
                if rand::thread_rng().gen::<f32>() > v {
                    chunk.graphics.set(x, y, Color::RGB(0, 0, 255)).unwrap();
                } else{
                    chunk.graphics.set(x, y, Color::RGB(0, 255, 0)).unwrap();
                }
            }
        }
    }
}