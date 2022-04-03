use crate::game::common::world::{
    material::{self, Color, MaterialInstance, PhysicsType},
    Chunk, CHUNK_SIZE,
};

use super::{ChunkContext, Populator};

pub struct NearbyReplacePopulator {
    pub radius: u16,
    pub matches: fn(&MaterialInstance) -> bool,
    pub replace_with: fn() -> MaterialInstance,
}

impl Populator<1> for NearbyReplacePopulator {
    fn populate(&self, mut chunks: ChunkContext<1>, _seed: i32) {
        // TODO: optimize this the same as the equivalent here: https://github.com/PieKing1215/FallingSandSurvival/blob/dev/FallingSandSurvival/Populators.cpp#L186=
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let m = chunks.get(x as i32, y as i32).unwrap();
                if m.material_id != material::AIR.id {
                    for dx in -(self.radius as i32)..=(self.radius as i32) {
                        for dy in -(self.radius as i32)..=(self.radius as i32) {
                            let m2 = chunks.get(x as i32 + dx, y as i32 + dy).unwrap();
                            if (self.matches)(m2) {
                                chunks
                                    .set(x as i32, y as i32, (self.replace_with)())
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
