use crate::game::{
    common::world::{material::MaterialInstance, CHUNK_SIZE},
    Registries,
};

use super::{ChunkContext, Populator};

pub struct SpawnPopulator;

impl Populator<0> for SpawnPopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<0>, _seed: i32, _registries: &Registries) {
        let (chunk_x, chunk_y) = chunks.center_chunk();
        let chunk_pixel_x = chunk_x * i32::from(CHUNK_SIZE);
        let chunk_pixel_y = chunk_y * i32::from(CHUNK_SIZE);

        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let wx = chunk_pixel_x + x;
                let wy = chunk_pixel_y + y;

                if wx.abs() < 150 && (wy + 32).abs() < 64 {
                    chunks.set(x, y, MaterialInstance::air()).unwrap();
                }
            }
        }
    }
}
