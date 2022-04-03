use crate::game::common::world::{Chunk, CHUNK_SIZE, material::{self, MaterialInstance, PhysicsType, Color}};

use super::{Populator, ChunkContext};


pub struct TestPopulator;

impl<const S: usize> Populator<S> for TestPopulator {
    fn populate(&self, mut chunks: ChunkContext<S>) {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let m = chunks.get(x as i32, y as i32).unwrap();
                if m.material_id != material::AIR.id {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let m2 = chunks.get(x as i32 + dx, y as i32 + dy).unwrap();
                            if m2.material_id == material::AIR.id {
                                chunks.set(x as i32, y as i32, MaterialInstance {
                                    material_id: material::TEST_MATERIAL.id,
                                    physics: PhysicsType::Solid,
                                    color: Color::ROSE,
                                }).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
