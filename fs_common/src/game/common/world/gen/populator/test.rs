use crate::game::common::{
    world::{
        material::{self, color::Color, PhysicsType},
        Chunk, CHUNK_SIZE,
    },
    Registries,
};

use super::{ChunkContext, Populator};

pub struct TestPopulator;

impl<const S: u8, C: Chunk> Populator<S, C> for TestPopulator {
    fn populate(&self, chunks: &mut ChunkContext<S, C>, _seed: i32, _registries: &Registries) {
        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let m = chunks.get(x, y).unwrap();
                if m.material_id != *material::AIR {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let m2 = chunks.get(x + dx, y + dy).unwrap();
                            if m2.material_id == *material::AIR {
                                chunks
                                    .set(
                                        x,
                                        y,
                                        material::TEST.instance(PhysicsType::Solid, Color::ROSE),
                                    )
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
