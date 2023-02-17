use crate::game::common::{
    world::{
        material::{placer, MaterialInstance},
        CHUNK_SIZE,
    },
    Registries,
};

use super::{ChunkContext, Populator};

pub struct SpawnPopulator;

impl Populator<0> for SpawnPopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<0>, _seed: i32, registries: &Registries) {
        let (chunk_x, chunk_y) = chunks.center_chunk();
        let chunk_pixel_x = i64::from(chunk_x) * i64::from(CHUNK_SIZE);
        let chunk_pixel_y = i64::from(chunk_y) * i64::from(CHUNK_SIZE);

        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let wx = chunk_pixel_x + i64::from(x);
                let wy = chunk_pixel_y + i64::from(y);

                if wx.abs() < 100 && (wy - 32).abs() < 4 {
                    chunks
                        .set(
                            x,
                            y,
                            registries
                                .material_placers
                                .get(&placer::COBBLE_STONE)
                                .unwrap()
                                .1
                                .pixel(wx, wy),
                        )
                        .unwrap();
                } else if wx.abs() < 150 && (wy + 32).abs() < 64 {
                    chunks.set(x, y, MaterialInstance::air()).unwrap();
                }
            }
        }
    }
}
