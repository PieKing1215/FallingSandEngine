
use liquidfun::box2d::dynamics::world::World;
use sdl2::{pixels::Color, rect::Rect};

use crate::game::common::world::material::{MaterialInstance, PhysicsType};
use crate::game::common::world::{CHUNK_SIZE, rigidbody};

use super::material::AIR;
use super::particle::Particle;
use super::rigidbody::RigidBody;
use super::{Chunk, ChunkHandler, ChunkHandlerGeneric, LIQUIDFUN_SCALE, Position, Velocity};
use super::gen::WorldGenerator;

pub struct Simulator {
    
}

trait SimulationHelper {
    unsafe fn get_pixel_local(&self, x: i32, y: i32) -> MaterialInstance;
    unsafe fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance);
    unsafe fn get_color_local(&self, x: i32, y: i32) -> Color;
    unsafe fn set_color_local(&mut self, x: i32, y: i32, col: Color);
    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity);
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
    particles: &'a mut Vec<(Particle, Position, Velocity)>,
    chunk_x: i32,
    chunk_y: i32,
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

    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity) {
        self.particles.push((Particle::of(material), Position {
            x: pos.x + self.chunk_x as f64 * f64::from(CHUNK_SIZE),
            y: pos.y + self.chunk_y as f64 * f64::from(CHUNK_SIZE),
        }, vel));
    }
}

struct SimulationHelperRigidBody<'a, T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> {
    chunk_handler: &'a mut ChunkHandler<T, C>,
    rigidbodies: &'a mut Vec<RigidBody>,
    particles: &'a mut Vec<(Particle, Position, Velocity)>,
}

impl <T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> SimulationHelper for SimulationHelperRigidBody<'_, T, C> {
    unsafe fn get_pixel_local(&self, x: i32, y: i32) -> MaterialInstance {
        let world_mat = self.chunk_handler.get(i64::from(x), i64::from(y)); // TODO: consider changing the args to i64
        if let Ok(m) = world_mat {
            if m.material_id != AIR.id {
                return *m;
            }
        }

        for i in 0..self.rigidbodies.len() {
            let cur = &self.rigidbodies[i];
            if let Some(body) = &cur.body {
                if body.is_active() {
                    let s = (-body.get_angle()).sin();
                    let c = (-body.get_angle()).cos();

                    let tx = x as f32 - body.get_position().x * LIQUIDFUN_SCALE;
                    let ty = y as f32 - body.get_position().y * LIQUIDFUN_SCALE;

                    let ntx = (tx * c - ty * s) as i32;
                    let nty = (tx * s + ty * c) as i32;

                    if ntx >= 0 && nty >= 0 && ntx < cur.width.into() && nty < cur.width.into() {
                        let px = cur.pixels[(ntx + nty * i32::from(cur.width)) as usize];

                        if px.material_id != AIR.id {
                            return px;
                        }
                    }

                }
            }
        }
        
        MaterialInstance::air()
    }

    unsafe fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        let _ignore = self.chunk_handler.set(i64::from(x), i64::from(y), mat); // TODO: consider changing the args to i64
    }

    unsafe fn get_color_local(&self, x: i32, y: i32) -> Color {
        let (chunk_x, chunk_y) = self.chunk_handler.pixel_to_chunk_pos(i64::from(x), i64::from(y));
        let chunk = self.chunk_handler.get_chunk(chunk_x, chunk_y);

        if let Some(ch) = chunk {
            let col_r = ch.get_color((i64::from(x) - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16, (i64::from(y) - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16);
            if let Ok(col) = col_r {
                if col.a > 0 {
                    return col;
                }
            }
        }

        for i in 0..self.rigidbodies.len() {
            let cur = &self.rigidbodies[i];
            if let Some(body) = &cur.body {
                if body.is_active() {
                    let s = (-body.get_angle()).sin();
                    let c = (-body.get_angle()).cos();

                    let tx = x as f32 - body.get_position().x * LIQUIDFUN_SCALE;
                    let ty = y as f32 - body.get_position().y * LIQUIDFUN_SCALE;

                    let ntx = (tx * c - ty * s) as i32;
                    let nty = (tx * s + ty * c) as i32;

                    if ntx >= 0 && nty >= 0 && ntx < cur.width.into() && nty < cur.width.into() {
                        let px = cur.pixels[(ntx + nty * i32::from(cur.width)) as usize];

                        if px.material_id != AIR.id {
                            return px.color;
                        }
                    }

                }
            }
        }
        
        Color::RGBA(0, 0, 0, 0)
    }

    unsafe fn set_color_local(&mut self, x: i32, y: i32, col: Color) {
        let (chunk_x, chunk_y) = self.chunk_handler.pixel_to_chunk_pos(i64::from(x), i64::from(y));
        let chunk = self.chunk_handler.get_chunk_mut(chunk_x, chunk_y);

        if let Some(ch) = chunk {
            let _ignore = ch.set_color((i64::from(x) - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16, (i64::from(y) - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16, col);
        }
    }

    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity) {
        self.particles.push((Particle::of(material), pos, vel));
    }
}

impl Simulator {
    #[profiling::function]
    pub fn simulate_chunk(chunk_x: i32, chunk_y: i32, pixels_raw: [usize; 9], colors_raw: [usize; 9], dirty: &mut [bool; 9], dirty_rects: &mut [Option<Rect>; 9], particles: &mut Vec<(Particle, Position, Velocity)>) {
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
                particles,
                chunk_x,
                chunk_y,
            };

            {
                profiling::scope!("");
                for y in (my_dirty_rect.y..(my_dirty_rect.y + my_dirty_rect.h) as i32).rev() {
                    for x in my_dirty_rect.x..(my_dirty_rect.x + my_dirty_rect.w) as i32 {
                        
                        let cur = helper.get_pixel_local(x, y);

                        if let Some(mat) = Self::simulate_pixel(x, y, cur, &mut helper) {
                            helper.set_color_local(x, y, mat.color);
                            helper.set_pixel_local(x, y, mat);
                        }
                    }
                }
            }

            helper.finish_dirty_rects();

        }
    }

    #[allow(clippy::unnecessary_unwrap)]
    #[allow(clippy::needless_range_loop)]
    pub fn simulate_rigidbodies<T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk>(chunk_handler: &mut ChunkHandler<T, C>, rigidbodies: &mut Vec<RigidBody>, lqf_world: &mut World, particles: &mut Vec<(Particle, Position, Velocity)>) {
        let mut dirty = vec![false; rigidbodies.len()];
        let mut needs_remesh = vec![false; rigidbodies.len()];
        for i in 0..rigidbodies.len() {

            let rb_w = rigidbodies[i].width;
            let rb_h = rigidbodies[i].height;
            let body_opt = rigidbodies[i].body.as_ref();

            if body_opt.is_some() {
                let s = body_opt.unwrap().get_angle().sin();
                let c = body_opt.unwrap().get_angle().cos();
                let pos_x = body_opt.unwrap().get_position().x * LIQUIDFUN_SCALE;
                let pos_y = body_opt.unwrap().get_position().y * LIQUIDFUN_SCALE;

                let mut helper = SimulationHelperRigidBody {
                    chunk_handler,
                    rigidbodies,
                    particles,
                };

                for rb_y in 0..rb_w {
                    for rb_x in 0..rb_h {
                        let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                        let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                        // let cur = helper.get_pixel_local(tx as i32, ty as i32);
                        let cur = helper.rigidbodies[i].pixels[(rb_x + rb_y * rb_w) as usize];

                        let res = Self::simulate_pixel(tx as i32, ty as i32, cur, &mut helper);

                        // if cur.material_id != AIR.id {
                        //     // helper.set_pixel_local(tx as i32, ty as i32, MaterialInstance {
                        //     //     material_id: TEST_MATERIAL.id,
                        //     //     physics: PhysicsType::Sand,
                        //     //     color: Color::RGB(64, 255, 64),
                        //     // });
                        //     // helper.set_pixel_local(tx as i32, ty as i32, cur);

                        // }

                        if let Some(mat) = res {
                            helper.rigidbodies[i].pixels[(rb_x + rb_y * rb_w) as usize] = mat;
                            dirty[i] = true;

                            if (cur.physics == PhysicsType::Solid && mat.physics != PhysicsType::Solid)
                                || (cur.physics != PhysicsType::Solid && mat.physics == PhysicsType::Solid) {
                                needs_remesh[i] = true;
                            }
                        }

                        // helper.rigidbodies[i].height = 5;
                    }
                }
            }
        }

        for i in 0..rigidbodies.len() {
            if dirty[i] && !needs_remesh[i] { // don't bother updating the image if it's going to be destroyed anyway
                rigidbodies[i].update_image();
            }
        }

        let mut new_rb: Vec<RigidBody> = rigidbodies.drain(..).enumerate().flat_map(|(i, rb): (usize, RigidBody)| {
            if needs_remesh[i] {
                let pos = (rb.body.as_ref().unwrap().get_position().x, rb.body.as_ref().unwrap().get_position().y);

                let b2_pos = rb.body.as_ref().unwrap().get_position();
                let b2_angle = rb.body.as_ref().unwrap().get_angle();
                let b2_linear_velocity = rb.body.as_ref().unwrap().get_linear_velocity();
                let b2_angular_velocity = rb.body.as_ref().unwrap().get_angular_velocity();

                // debug!("#bodies before = {}", lqf_world.get_body_count());
                lqf_world.destroy_body(rb.body.as_ref().unwrap());
                // debug!("#bodies after  = {}", lqf_world.get_body_count());
                let mut r = rigidbody::RigidBody::make_bodies(&rb.pixels, rb.width, rb.height, lqf_world, pos).unwrap_or_default();
                // debug!("#bodies after2 = {} new pos = {:?}", lqf_world.get_body_count(), r[0].body.as_ref().unwrap().get_position());

                for rb in &mut r {
                    rb.body.as_mut().unwrap().set_transform(b2_pos, b2_angle);
                    rb.body.as_mut().unwrap().set_linear_velocity(b2_linear_velocity);
                    rb.body.as_mut().unwrap().set_angular_velocity(b2_angular_velocity);
                }

                r
            } else {
                vec![rb]
            }
        }).collect();

        rigidbodies.append(&mut new_rb);

    }

    fn simulate_pixel(x: i32, y: i32, cur: MaterialInstance, helper: &mut dyn SimulationHelper) -> Option<MaterialInstance> {
        unsafe {
            let mut new_mat = None;

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

                        let empty_below = (0..4).all(|i| {
                            let pix = helper.get_pixel_local(x, y + i + 2); // don't include myself or one below
                            pix.physics == PhysicsType::Air
                        });

                        if empty_below {
                            helper.add_particle(cur,
                                Position{ 
                                    x: x as f64, 
                                    y: y as f64 
                                }, 
                                Velocity { 
                                    x: (rand::random::<f64>() - 0.5) * 0.5, 
                                    y: 1.0 + rand::random::<f64>() 
                                });
                        } else {
                            helper.set_color_local(x, y + 1, cur.color);
                            helper.set_pixel_local(x, y + 1, cur);
                        }
                        
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
