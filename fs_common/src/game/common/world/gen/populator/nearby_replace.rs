use crate::game::common::world::{material::MaterialInstance, CHUNK_SIZE};

use super::{ChunkContext, Populator};

pub struct NearbyReplacePopulator {
    pub radius: u16,
    pub replace: fn(&MaterialInstance) -> Option<MaterialInstance>,
    pub searching_for: fn(&MaterialInstance) -> bool,
}

impl Populator<1> for NearbyReplacePopulator {
    #[profiling::function]
    fn populate(&self, mut chunks: ChunkContext<1>, _seed: i32) {
        // the skip_x and skip_y stuff helps avoid a lot of redundant pixel checks
        // otherwise this is basically just brute force
        // for each pixel that matches `searching_for`, scan around it and try to `replace`

        let mut skip_y = [0; CHUNK_SIZE as usize];

        for y in 0..i32::from(CHUNK_SIZE) {
            let mut skip_x = 0;
            for x in 0..i32::from(CHUNK_SIZE) {
                let m = chunks.get(x as i32, y as i32).unwrap();
                if (self.searching_for)(m) {
                    let range = i32::from(self.radius);
                    for dx in (-range + i32::from(skip_x))..=range {
                        for dy in (-range + i32::from(skip_y[x as usize]))..=range {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let m2 = chunks.get(x as i32 + dx, y as i32 + dy).unwrap();
                            if let Some(rep) = (self.replace)(m2) {
                                chunks.set(x as i32 + dx, y as i32 + dy, rep).unwrap();
                            }
                        }
                    }

                    skip_x = self.radius * 2;
                    skip_y[x as usize] = self.radius * 2;
                } else if skip_x > 0 {
                    skip_x -= 1;
                }

                if skip_y[x as usize] > 0 {
                    skip_y[x as usize] -= 1;
                }
            }
        }
    }
}
