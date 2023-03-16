use crate::game::common::{
    world::{material::MaterialInstance, Chunk, CHUNK_SIZE},
    Registries,
};

use super::{ChunkContext, Populator};

pub struct NearbyReplacePopulator<
    R: Fn(&MaterialInstance, i64, i64, &Registries) -> Option<MaterialInstance>,
    S: Fn(&MaterialInstance) -> bool,
> {
    pub radius: u16,
    pub replace: R,
    pub searching_for: S,
}

impl<
        R: Fn(&MaterialInstance, i64, i64, &Registries) -> Option<MaterialInstance>,
        S: Fn(&MaterialInstance) -> bool,
        C: Chunk,
    > Populator<1, C> for NearbyReplacePopulator<R, S>
{
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<1, C>, _seed: i32, registries: &Registries) {
        // the skip_x and skip_y stuff helps avoid a lot of redundant pixel checks
        // otherwise this is basically just brute force
        // for each pixel that matches `searching_for`, scan around it and try to `replace`

        let cofs_x = i64::from(chunks.center_chunk().0) * i64::from(CHUNK_SIZE);
        let cofs_y = i64::from(chunks.center_chunk().1) * i64::from(CHUNK_SIZE);

        const OVERSCAN: u16 = 4;

        // const SIDE: usize = CHUNK_SIZE as usize + (OVERSCAN as usize * 2);
        // let mut mask = [false; SIDE * SIDE];

        // {
        //     profiling::scope!("build mask");
        //     let range = i32::from(self.radius);
        //     for y in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
        //         for x in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
        //             // let i = (x + i32::from(OVERSCAN)) as usize + (y + i32::from(OVERSCAN)) as usize * SIDE;
        //             // if !mask[i] {
        //             let m = unsafe { chunks.get(x, y).unwrap_unchecked() };
        //             if (self.searching_for)(m) {
        //                 for dx in -range..=range {
        //                     for dy in -range..=range {
        //                         let i = (x + dx + i32::from(OVERSCAN)) as usize + (y + dy + i32::from(OVERSCAN)) as usize * SIDE;
        //                         if let Some(v) = mask.get_mut(i) {
        //                             *v = true;
        //                         }
        //                     }
        //                 }
        //             }
        //             // }
        //         }
        //     }
        // }

        // {
        //     profiling::scope!("loop");
        //     for x in 0..CHUNK_SIZE {
        //         for y in 0..CHUNK_SIZE {
        //             let i = (x as i32 + i32::from(OVERSCAN)) as usize + (y as i32 + i32::from(OVERSCAN)) as usize * SIDE;
        //             if unsafe { *mask.get_unchecked(i) } {
        //                 chunks.replace(x, y, |mat| {
        //                     (self.replace)(
        //                         mat,
        //                         cofs_x + i64::from(x),
        //                         cofs_y + i64::from(y),
        //                         registries,
        //                     )
        //                 });
        //             }
        //         }
        //     }
        // }

        let mut skip_y = [0; CHUNK_SIZE as usize + OVERSCAN as usize * 2];
        {
            profiling::scope!("loop");
            let range = i32::from(self.radius);
            for y in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
                let mut skip_x = 0;
                for x in -i32::from(OVERSCAN)..i32::from(CHUNK_SIZE) + i32::from(OVERSCAN) {
                    let m = unsafe { chunks.get(x, y).unwrap_unchecked() };
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
