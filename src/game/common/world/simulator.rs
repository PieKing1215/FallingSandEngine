use sdl2::{pixels::Color, rect::Rect};

use crate::game::common::world::material::{MaterialInstance, PhysicsType};
use crate::game::common::world::CHUNK_SIZE;

pub struct Simulator {
    
}

trait SimulationHelper {
    unsafe fn get_pixel_local(&self, x: i32, y: i32) -> MaterialInstance;
    unsafe fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance);
    unsafe fn get_color_local(&self, x: i32, y: i32) -> Color;
    unsafe fn set_color_local(&mut self, x: i32, y: i32, col: Color);
}

struct SimulationHelperChunk<'a> {
    pixels: [*mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]; 9],
    colors: [*mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]; 9],
    dirty: &'a mut [bool; 9], 
    dirty_rects: &'a mut [Option<Rect>; 9],
    min_x: [u16; 9],
    min_y: [u16; 9],
    max_x: [u16; 9],
    max_y: [u16; 9],
}

impl SimulationHelperChunk<'_> {
    unsafe fn get_pixel_from_index(&self, (ch, px, ..): (usize, usize, u16, u16)) -> MaterialInstance {
        (*self.pixels[ch])[px]
    }

    unsafe fn set_pixel_from_index(&mut self, (ch, px, ch_x, ch_y): (usize, usize, u16, u16), mat: MaterialInstance) {
        (*self.pixels[ch])[px] = mat;

        self.min_x[ch] = self.min_x[ch].min(ch_x);
        self.min_y[ch] = self.min_y[ch].min(ch_y);
        self.max_x[ch] = self.max_x[ch].max(ch_x);
        self.max_y[ch] = self.max_y[ch].max(ch_y);
    }

    unsafe fn get_color_from_index(&self, (ch, px, ..): (usize, usize, u16, u16)) -> Color {
        Color::RGBA(
            (*self.colors[ch])[px * 4    ],
            (*self.colors[ch])[px * 4 + 1],
            (*self.colors[ch])[px * 4 + 2],
            (*self.colors[ch])[px * 4 + 3],
        )
    }

    unsafe fn set_color_from_index(&mut self, (ch, px, ..): (usize, usize, u16, u16), color: Color) {
        (*self.colors[ch])[px * 4    ] = color.r;
        (*self.colors[ch])[px * 4 + 1] = color.g;
        (*self.colors[ch])[px * 4 + 2] = color.b;
        (*self.colors[ch])[px * 4 + 3] = color.a;

        self.dirty[ch] = true;
    }

    // (chunk index, pixel index, pixel x in chunk, pixel y in chunk)
    fn local_to_indices(x: i32, y: i32) -> (usize, usize, u16, u16) {
        let size = i32::from(CHUNK_SIZE);
        let rel_chunk_x = (x as f32 / f32::from(CHUNK_SIZE)).floor() as i8;
        let rel_chunk_y = (y as f32 / f32::from(CHUNK_SIZE)).floor() as i8;
        
        let chunk_px_x = x.rem_euclid(size) as u16;
        let chunk_px_y = y.rem_euclid(size) as u16;

        ((rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3, (chunk_px_x + chunk_px_y * CHUNK_SIZE) as usize, chunk_px_x, chunk_px_y)
    }

    fn finish_dirty_rects(&mut self) {
        for i in 0..9 {
            if self.min_x[i] == CHUNK_SIZE + 1 {
                self.dirty_rects[i] = None;
            }else{
                self.dirty_rects[i] = Some(Rect::new(i32::from(self.min_x[i]), i32::from(self.min_y[i]), u32::from(self.max_x[i] - self.min_x[i]) + 1, u32::from(self.max_y[i] - self.min_y[i]) + 1));
            }
        }
    }
}

impl SimulationHelper for SimulationHelperChunk<'_> {
    unsafe fn get_pixel_local(&self, x: i32, y: i32) -> MaterialInstance {
        self.get_pixel_from_index(Self::local_to_indices(x, y))
    }

    unsafe fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        self.set_pixel_from_index(Self::local_to_indices(x, y), mat);
    }

    unsafe fn get_color_local(&self, x: i32, y: i32) -> Color {
        self.get_color_from_index(Self::local_to_indices(x, y))
    }

    unsafe fn set_color_local(&mut self, x: i32, y: i32, col: Color) {
        self.set_color_from_index(Self::local_to_indices(x, y), col);
    }
}

impl Simulator {
    #[profiling::function]
    pub fn simulate_chunk(_chunk_x: i32, _chunk_y: i32, pixels_raw: [usize; 9], colors_raw: [usize; 9], dirty: &mut [bool; 9], dirty_rects: &mut [Option<Rect>; 9]) {
        const CENTER_CHUNK: usize = 4;

        let my_dirty_rect_o = dirty_rects[CENTER_CHUNK];
        if my_dirty_rect_o.is_none() {
            dirty_rects.fill(None);
            return;
        }
        let my_dirty_rect = my_dirty_rect_o.unwrap();


        unsafe {
            let mut helper = SimulationHelperChunk {
                pixels: [
                    &mut *(pixels_raw[0] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[1] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[2] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[3] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[4] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[5] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[6] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[7] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                    &mut *(pixels_raw[8] as *mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]),
                ],
                colors: [
                    &mut *(colors_raw[0] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[1] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[2] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[3] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[4] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[5] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[6] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[7] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                    &mut *(colors_raw[8] as *mut [u8; (CHUNK_SIZE * CHUNK_SIZE) as usize * 4]),
                ],
                dirty,
                dirty_rects,
                min_x: [CHUNK_SIZE + 1; 9],
                min_y: [CHUNK_SIZE + 1; 9],
                max_x: [0; 9],
                max_y: [0; 9],
            };

            {
                profiling::scope!("");
                for y in (my_dirty_rect.y..(my_dirty_rect.y + my_dirty_rect.h) as i32).rev() {
                    for x in my_dirty_rect.x..(my_dirty_rect.x + my_dirty_rect.w) as i32 {
                        if let Some(mat) = Self::simulate_pixel(x, y, &mut helper) {
                            helper.set_color_local(x, y, mat.color);
                            helper.set_pixel_local(x, y, mat);
                        }
                    }
                }
            }

            helper.finish_dirty_rects();

        }
    }

    fn simulate_pixel(x: i32, y: i32, helper: &mut dyn SimulationHelper) -> Option<MaterialInstance> {
        unsafe {
            let cur = helper.get_pixel_local(x, y);
            let mut new_mat: Option<MaterialInstance> = None;

            #[allow(clippy::single_match)]
            match cur.physics {
                PhysicsType::Sand => {
                    let below = helper.get_pixel_local(x, y + 1);
                    let below_can = below.physics == PhysicsType::Air;

                    let bl = helper.get_pixel_local(x - 1, y + 1);
                    let bl_can = bl.physics == PhysicsType::Air;

                    let br = helper.get_pixel_local(x + 1, y + 1);
                    let br_can = br.physics == PhysicsType::Air;
                    
                    if below_can && (!(br_can || bl_can) || rand::random::<f32>() > 0.1) {
                        // let below2_i = index_helper(x, y + 2);
                        // let below2 = (*pixels[below_i.0])[below_i.1];
                        // if below2.physics == PhysicsType::Air {
                        //     set_color(x, y + 2, cur.color, true);
                        //     (*pixels[below2_i.0])[below2_i.1] = cur;
                        //     new_mat = Some(MaterialInstance::air());
                        // }else {
                            helper.set_color_local(x, y + 1, cur.color);
                            helper.set_pixel_local(x, y + 1, cur);
                            new_mat = Some(MaterialInstance::air());
                        // }
                    }else if bl_can && br_can {
                        if rand::random::<bool>() {
                            helper.set_color_local(x + 1, y + 1, cur.color);
                            helper.set_pixel_local(x + 1, y + 1, cur);
                        }else{
                            helper.set_color_local(x - 1, y + 1, cur.color);
                            helper.set_pixel_local(x - 1, y + 1, cur);
                        }
                        new_mat = Some(MaterialInstance::air());
                    }else if bl_can {
                        helper.set_color_local(x - 1, y + 1, cur.color);
                        helper.set_pixel_local(x - 1, y + 1, cur);
                        new_mat = Some(MaterialInstance::air());
                    }else if br_can {
                        helper.set_color_local(x + 1, y + 1, cur.color);
                        helper.set_pixel_local(x + 1, y + 1, cur);
                        new_mat = Some(MaterialInstance::air());
                    }
                },
                _ => {},
            }

            new_mat
        }
    }
}
