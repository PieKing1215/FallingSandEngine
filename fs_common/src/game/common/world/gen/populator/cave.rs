use simdnoise::NoiseBuilder;

use crate::game::common::world::{material::MaterialInstance, CHUNK_SIZE};

use super::{ChunkContext, Populator};

pub struct CavePopulator;

impl Populator<0> for CavePopulator {
    #[profiling::function]
    fn populate(&self, mut chunks: ChunkContext<0>, seed: i32) {
        let (chunk_x, chunk_y) = chunks.center_chunk();
        let chunk_pixel_x = chunk_x * i32::from(CHUNK_SIZE);
        let chunk_pixel_y = chunk_y * i32::from(CHUNK_SIZE);
        let cofs_x = chunk_pixel_x as f32;
        let cofs_y = chunk_pixel_y as f32;

        let enable_caves = true;

        if enable_caves {
            let noise_cave_2 = NoiseBuilder::gradient_2d_offset(
                cofs_x,
                CHUNK_SIZE.into(),
                cofs_y,
                CHUNK_SIZE.into(),
            )
            .with_freq(0.002)
            .with_seed(seed)
            .generate()
            .0;

            for x in 0..i32::from(CHUNK_SIZE) {
                for y in 0..i32::from(CHUNK_SIZE) {
                    let i = (x + y * i32::from(CHUNK_SIZE)) as usize;
                    if noise_cave_2[i] > 0.0 {
                        chunks
                            .set(x as i32, y as i32, MaterialInstance::air())
                            .unwrap();
                    }
                }
            }
        }
    }
}
