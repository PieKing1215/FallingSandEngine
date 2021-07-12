use sdl2::{pixels::Color, rect::Rect};

use crate::game::world::{PhysicsType, TEST_MATERIAL};

use super::{CHUNK_SIZE, MaterialInstance};


pub struct Simulator {
    
}

impl Simulator {
    #[profiling::function]
    pub fn simulate_chunk(chunk_x: i32, chunk_y: i32, pixels_raw: [usize; 9], colors_raw: [usize; 9], dirty: &mut [bool; 9], dirty_rects: &mut [Option<Rect>; 9]) {
        const CENTER_CHUNK: usize = 4;

        let my_dirty_rect_o = dirty_rects[CENTER_CHUNK];
        if my_dirty_rect_o.is_none() {
            dirty_rects.fill(None);
            return;
        }
        let my_dirty_rect = my_dirty_rect_o.unwrap();

        let mut min_x = [CHUNK_SIZE + 1; 9];
        let mut min_y = [CHUNK_SIZE + 1; 9];
        let mut max_x = [0; 9];
        let mut max_y = [0; 9];

        unsafe {
            let pixels: [*mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]; 9] = [
                &mut *(pixels_raw[0] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[1] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[2] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[3] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[4] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[5] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[6] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[7] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                &mut *(pixels_raw[8] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
            ];

            let colors: [*mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]; 9] = [
                &mut *(colors_raw[0] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[1] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[2] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[3] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[4] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[5] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[6] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[7] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                &mut *(colors_raw[8] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
            ];

            // let mut pix = |x: i32, y: i32| {
            //     let size = CHUNK_SIZE as i32;
            //     // if x < -size || y < -size || x >= 2 * size || y >= 2 * size {
            //     //     return Err("Chunk index out of bounds");
            //     // }
            //     let rel_chunk_x = (x as f32 / CHUNK_SIZE as f32).floor() as i8;
            //     let rel_chunk_y = (y as f32 / CHUNK_SIZE as f32).floor() as i8;
                
            //     let chunk_px_x = x.rem_euclid(size) as usize;
            //     let chunk_px_y = y.rem_euclid(size) as usize;

            //     &mut pixels[(rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3][chunk_px_x + chunk_px_y * CHUNK_SIZE as usize]

            //     // return Ok(());
            // };

            let index_helper = |x: i32, y: i32| {
                let size = CHUNK_SIZE as i32;
                let rel_chunk_x = (x as f32 / CHUNK_SIZE as f32).floor() as i8;
                let rel_chunk_y = (y as f32 / CHUNK_SIZE as f32).floor() as i8;
                
                let chunk_px_x = x.rem_euclid(size) as u16;
                let chunk_px_y = y.rem_euclid(size) as u16;

                ((rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3, (chunk_px_x + chunk_px_y * CHUNK_SIZE) as usize, chunk_px_x, chunk_px_y)
            };

            let mut set_color = |x: i32, y: i32, Color { r, g, b, a }: Color, graphics: bool| {
                let i = index_helper(x, y);

                if graphics {
                    (*colors[i.0])[i.1 * 4 + 3] = a;
                    (*colors[i.0])[i.1 * 4 + 2] = r;
                    (*colors[i.0])[i.1 * 4 + 1] = g;
                    (*colors[i.0])[i.1 * 4 + 0] = b;

                    dirty[i.0] = true;
                }

                min_x[i.0] = min_x[i.0].min(i.2);
                min_y[i.0] = min_y[i.0].min(i.3);
                max_x[i.0] = max_x[i.0].max(i.2);
                max_y[i.0] = max_y[i.0].max(i.3);
            };

            {
                profiling::scope!("");
                for y in (my_dirty_rect.y..(my_dirty_rect.y + my_dirty_rect.h) as i32).rev() {
                    for x in my_dirty_rect.x..(my_dirty_rect.x + my_dirty_rect.w) as i32 {
                        let i = index_helper(x, y);
                        let cur = (*pixels[i.0])[i.1];
                        let mut new_mat: Option<MaterialInstance> = None;

                        match cur.physics {
                            crate::game::world::PhysicsType::Sand => {
                                let below_i = index_helper(x, y + 1);
                                let below = (*pixels[below_i.0])[below_i.1];
                                let below_can = below.physics == PhysicsType::Air;

                                let bl_i = index_helper(x - 1, y + 1);
                                let bl = (*pixels[bl_i.0])[bl_i.1];
                                let bl_can = bl.physics == PhysicsType::Air;

                                let br_i = index_helper(x + 1, y + 1);
                                let br = (*pixels[br_i.0])[br_i.1];
                                let br_can = br.physics == PhysicsType::Air;
                                
                                if below_can && (!(br_can || bl_can) || rand::random::<f32>() > 0.1) {
                                    // let below2_i = index_helper(x, y + 2);
                                    // let below2 = (*pixels[below_i.0])[below_i.1];
                                    // if below2.physics == PhysicsType::Air {
                                    //     set_color(x, y + 2, cur.color, true);
                                    //     (*pixels[below2_i.0])[below2_i.1] = cur;
                                    //     new_mat = Some(MaterialInstance::air());
                                    // }else {
                                        set_color(x, y + 1, cur.color, true);
                                        (*pixels[below_i.0])[below_i.1] = cur;
                                        new_mat = Some(MaterialInstance::air());
                                    // }
                                }else {
                                    if bl_can && br_can {
                                        if rand::random::<bool>() {
                                            set_color(x + 1, y + 1, cur.color, true);
                                            (*pixels[br_i.0])[br_i.1] = cur;
                                            new_mat = Some(MaterialInstance::air());
                                        }else{
                                            set_color(x - 1, y + 1, cur.color, true);
                                            (*pixels[bl_i.0])[bl_i.1] = cur;
                                            new_mat = Some(MaterialInstance::air());
                                        }
                                    }else if bl_can {
                                        set_color(x - 1, y + 1, cur.color, true);
                                        (*pixels[bl_i.0])[bl_i.1] = cur;
                                        new_mat = Some(MaterialInstance::air());
                                    }else if br_can {
                                        set_color(x + 1, y + 1, cur.color, true);
                                        (*pixels[br_i.0])[br_i.1] = cur;
                                        new_mat = Some(MaterialInstance::air());
                                    }
                                }
                            },
                            _ => {},
                        }

                        if let Some(m) = new_mat {
                            set_color(x, y, m.color, true);
                            (*pixels[i.0])[i.1] = m;
                        }
                    }
                }
            }

            for i in 0..9 {
                if min_x[i] == CHUNK_SIZE + 1 {
                    dirty_rects[i] = None;
                }else{
                    dirty_rects[i] = Some(Rect::new(min_x[i] as i32, min_y[i] as i32, (max_x[i] - min_x[i]) as u32 + 1, (max_y[i] - min_y[i]) as u32 + 1));
                }
            }

        }
    }
}
