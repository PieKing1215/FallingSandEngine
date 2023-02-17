use crate::game::common::{
    world::{material::MaterialInstance, CHUNK_SIZE},
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

        {
            profiling::scope!("loop");
            let range = i32::from(self.radius);
            for y in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
                let mut skip_x = 0;
                for x in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
                    let m = chunks.get(x, y).unwrap();
                    let cur_skip_y =
                        unsafe { skip_y.get_unchecked_mut((x + i32::from(OVERSCAN)) as usize) };
                    if (self.searching_for)(m) {
                        for dx in (-range + i32::from(skip_x))..=range {
                            if x + dx < 0 {
                                continue;
                            }
                            if x + dx >= i32::from(CHUNK_SIZE) {
                                break;
                            }
                            for dy in (-range + i32::from(*cur_skip_y))..=range {
                                if (dx == 0 && dy == 0) || y + dy < 0 {
                                    continue;
                                }
                                if y + dy >= i32::from(CHUNK_SIZE) {
                                    break;
                                }
                                chunks.replace(x + dx, y + dy, |mat| {
                                    (self.replace)(
                                        mat,
                                        cofs_x + i64::from(x + dx),
                                        cofs_y + i64::from(y + dy),
                                        registries,
                                    )
                                });
                            }
                        }

                        skip_x = self.radius * 2;
                        *cur_skip_y = self.radius * 2;
                    } else if skip_x > 0 {
                        skip_x -= 1;
                    }

                    if *cur_skip_y > 0 {
                        *cur_skip_y -= 1;
                    }
                }
            }
        }
    }
}
