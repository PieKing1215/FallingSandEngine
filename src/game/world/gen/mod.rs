mod test;

use std::usize;

pub use test::*;

use super::{CHUNK_SIZE, Chunk, MaterialInstance};

pub trait WorldGenerator {
    fn generate(&self, chunk_x: i32, chunk_y: i32, pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize], colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]);

    fn max_gen_stage(&self) -> u8;
}

// unsafe impl<T> Send for T where T: WorldGenerator {}