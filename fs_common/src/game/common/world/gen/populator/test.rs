use crate::game::common::{
    world::{
        material::{self, color::Color, MaterialInstance, PhysicsType},
        CHUNK_SIZE,
    },
    Registries,
};

use super::{ChunkContext, Populator};

pub struct TestPopulator;

impl<const S: u8> Populator<S> for TestPopulator {
    fn populate(&self, chunks: &mut ChunkContext<S>, _seed: i32, _registries: &Registries) {
        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let m = chunks.get(x, y).unwrap();
                if m.material_id != material::AIR {
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            let m2 = chunks.get(x + dx, y + dy).unwrap();
                            if m2.material_id == material::AIR {
                                chunks
                                    .set(
                                        x,
                                        y,
                                        MaterialInstance {
                                            material_id: material::TEST.to_string(),
                                            physics: PhysicsType::Solid,
                                            color: Color::ROSE,
                                        },
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
