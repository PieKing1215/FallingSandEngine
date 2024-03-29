use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::game::common::{
    world::{
        material::{self, MaterialInstance},
        Chunk, CHUNK_SIZE,
    },
    Registries,
};

use super::{ChunkContext, Populator};

pub struct StalactitePopulator {
    pub searching_for: fn(&MaterialInstance) -> bool,
    pub replace: fn(&MaterialInstance, i64, i64, &Registries) -> Option<MaterialInstance>,
}

impl<C: Chunk> Populator<1, C> for StalactitePopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<1, C>, seed: i32, registries: &Registries) {
        let mut rng = StdRng::seed_from_u64(seed as u64);

        'skip: for _ in 0..1000 {
            let x = rng.gen_range(0..i32::from(CHUNK_SIZE));
            let y = rng.gen_range(0..i32::from(CHUNK_SIZE));

            let m = chunks.get(x, y).unwrap();
            if (self.searching_for)(m)
                && chunks.get(x, y + 1).unwrap().material_id == *material::AIR
            {
                for dx in -4..=4 {
                    for dy in 0..=2 {
                        if chunks.get(x + dx, y - dy).unwrap().material_id == *material::AIR {
                            continue 'skip;
                        }
                    }
                }

                let height = rng.gen_range(16..=24);
                let dx_per_dy = rng.gen_range(-0.15..=0.15);
                for dy in -2..height {
                    let size = (height - dy.max(0)) / 5 + 1;
                    for dx in -size..=size {
                        let dx = dx + (dy as f32 * dx_per_dy) as i32;
                        let m2 = chunks.get(x + dx, y + dy).unwrap();
                        if let Some(rep) = (self.replace)(
                            m2,
                            (i64::from(chunks.center_chunk().0) * i64::from(CHUNK_SIZE))
                                + i64::from(x)
                                + i64::from(dx),
                            (i64::from(chunks.center_chunk().1) * i64::from(CHUNK_SIZE))
                                + i64::from(y)
                                + i64::from(dy),
                            registries,
                        ) {
                            chunks.set(x + dx, y + dy, rep).unwrap();
                        }
                    }
                }
                return;
            }
        }
    }
}
