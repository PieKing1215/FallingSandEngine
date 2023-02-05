use crate::game::{
    common::world::{material::MaterialInstance, CHUNK_SIZE},
    Registries,
};

use super::{ChunkContext, Populator};

pub struct NearbyReplacePopulator {
    pub radius: u16,
    pub replace: fn(&MaterialInstance, i64, i64, &Registries) -> Option<MaterialInstance>,
    pub searching_for: fn(&MaterialInstance) -> bool,
}

impl Populator<1> for NearbyReplacePopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<1>, _seed: i32, registries: &Registries) {
        // the skip_x and skip_y stuff helps avoid a lot of redundant pixel checks
        // otherwise this is basically just brute force
        // for each pixel that matches `searching_for`, scan around it and try to `replace`

        let cofs_x = i64::from(chunks.center_chunk().0) * i64::from(CHUNK_SIZE);
        let cofs_y = i64::from(chunks.center_chunk().1) * i64::from(CHUNK_SIZE);

        const OVERSCAN: u16 = 4;

        let mut skip_y = [0; CHUNK_SIZE as usize + OVERSCAN as usize * 2];

        for y in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
            let mut skip_x = 0;
            // log::trace!("{} {} {}", i32::from(OVERSCAN), i32::from(CHUNK_SIZE), i32::from(CHUNK_SIZE) + i32::from(OVERSCAN));
            for x in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
                // log::trace!("{} / {}", x, x as usize);
                let m = chunks.get(x, y).unwrap();
                if (self.searching_for)(m) {
                    let range = i32::from(self.radius);
                    for dx in (-range + i32::from(skip_x))..=range {
                        if x + dx < 0 || x + dx >= i32::from(CHUNK_SIZE) {
                            continue;
                        }
                        // log::trace!("{} + {}", x as usize, OVERSCAN as usize);
                        for dy in
                            (-range + i32::from(skip_y[(x + i32::from(OVERSCAN)) as usize]))..=range
                        {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            if y + dy < 0 || y + dy >= i32::from(CHUNK_SIZE) {
                                continue;
                            }
                            let m2 = chunks.get(x + dx, y + dy).unwrap();
                            if let Some(rep) = (self.replace)(
                                m2,
                                cofs_x + i64::from(x) + i64::from(dx),
                                cofs_y + i64::from(y) + i64::from(dy),
                                registries,
                            ) {
                                chunks.set(x + dx, y + dy, rep).unwrap();
                            }
                        }
                    }

                    skip_x = self.radius * 2;
                    skip_y[(x + i32::from(OVERSCAN)) as usize] = self.radius * 2;
                } else if skip_x > 0 {
                    skip_x -= 1;
                }

                if skip_y[(x + i32::from(OVERSCAN)) as usize] > 0 {
                    skip_y[(x + i32::from(OVERSCAN)) as usize] -= 1;
                }
            }
        }
    }
}
