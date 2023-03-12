use std::cell::UnsafeCell;
use std::sync::Arc;

use chunksystem::ChunkQuery;
use fastrand::Rng;
use rapier2d::na::Isometry2;

use crate::game::common::world::material::{MaterialInstance, PhysicsType};
use crate::game::common::world::{rigidbody, CHUNK_SIZE};
use crate::game::common::{Rect, Registries};

use super::chunk_access::FSChunkAccess;
use super::material::color::Color;
use super::particle::Particle;
use super::rigidbody::FSRigidBody;
use super::{material, pixel_to_chunk_pos};
use super::{
    physics::{Physics, PHYSICS_SCALE},
    Chunk, ChunkHandler, Position, Velocity,
};

pub struct Simulator {}

trait SimulationHelper {
    fn pixel_local(&self, x: i32, y: i32) -> &MaterialInstance;
    fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance);
    fn color_local(&self, x: i32, y: i32) -> Color;
    fn set_color_local(&mut self, x: i32, y: i32, col: Color);
    fn light_local(&self, x: i32, y: i32) -> &[f32; 3];
    fn set_light_local(&mut self, x: i32, y: i32, light: [f32; 3]);

    fn set_all_local(&mut self, x: i32, y: i32, mat: MaterialInstance);
    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity);
}

struct SimulationHelperChunk<'a, 'b> {
    chunk_data: &'a mut [SimulatorChunkContext<'b>; 9],
    min_x: [u16; 9],
    min_y: [u16; 9],
    max_x: [u16; 9],
    max_y: [u16; 9],
    particles: &'a mut Vec<Particle>,
    chunk_x: i32,
    chunk_y: i32,
}

#[allow(unused)]
impl SimulationHelperChunk<'_, '_> {
    #[inline]
    fn pixel_from_index(&self, (ch, px, ..): (usize, usize, u16, u16)) -> &MaterialInstance {
        unsafe { &*self.chunk_data[ch].pixels[px].get() }
    }

    #[inline]
    unsafe fn pixel_from_index_unchecked(
        &self,
        (ch, px, ..): (usize, usize, u16, u16),
    ) -> &MaterialInstance {
        &*self
            .chunk_data
            .get_unchecked(ch)
            .pixels
            .get_unchecked(px)
            .get()
    }

    #[inline(always)]
    unsafe fn pixel_local_unchecked(&self, x: i32, y: i32) -> &MaterialInstance {
        self.pixel_from_index_unchecked(Self::local_to_indices(x, y))
    }

    #[inline]
    fn set_pixel_from_index(
        &mut self,
        (ch, px, ch_x, ch_y): (usize, usize, u16, u16),
        mat: MaterialInstance,
    ) {
        unsafe {
            *self.chunk_data[ch].pixels[px].get() = mat;
        }

        self.min_x[ch] = self.min_x[ch].min(ch_x);
        self.min_y[ch] = self.min_y[ch].min(ch_y);
        self.max_x[ch] = self.max_x[ch].max(ch_x);
        self.max_y[ch] = self.max_y[ch].max(ch_y);
    }

    #[inline]
    unsafe fn set_pixel_from_index_unchecked(
        &mut self,
        (ch, px, ch_x, ch_y): (usize, usize, u16, u16),
        mat: MaterialInstance,
    ) {
        *self
            .chunk_data
            .get_unchecked_mut(ch)
            .pixels
            .get_unchecked(px)
            .get() = mat;

        *self.min_x.get_unchecked_mut(ch) = (*self.min_x.get_unchecked_mut(ch)).min(ch_x);
        *self.min_y.get_unchecked_mut(ch) = (*self.min_y.get_unchecked_mut(ch)).min(ch_y);
        *self.max_x.get_unchecked_mut(ch) = (*self.max_x.get_unchecked_mut(ch)).max(ch_x);
        *self.max_y.get_unchecked_mut(ch) = (*self.max_y.get_unchecked_mut(ch)).max(ch_y);
    }

    #[inline]
    unsafe fn set_pixel_local_unchecked(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        self.set_pixel_from_index_unchecked(Self::local_to_indices(x, y), mat);
    }

    #[inline]
    fn color_from_index(&self, (ch, px, ..): (usize, usize, u16, u16)) -> Color {
        unsafe { *self.chunk_data[ch].colors[px].get() }
    }

    #[inline]
    #[allow(dead_code)]
    unsafe fn color_from_index_unchecked(&self, (ch, px, ..): (usize, usize, u16, u16)) -> Color {
        *self
            .chunk_data
            .get_unchecked(ch)
            .colors
            .get_unchecked(px)
            .get()
    }

    #[inline]
    #[allow(dead_code)]
    unsafe fn color_local_unchecked(&self, x: i32, y: i32) -> Color {
        self.color_from_index_unchecked(Self::local_to_indices(x, y))
    }

    #[inline]
    fn set_color_from_index(&mut self, (ch, px, ..): (usize, usize, u16, u16), color: Color) {
        unsafe {
            *self.chunk_data[ch].colors[px].get() = color;
        }

        self.chunk_data[ch].dirty = true;
    }

    #[inline]
    unsafe fn set_color_from_index_unchecked(
        &mut self,
        (ch, px, ..): (usize, usize, u16, u16),
        color: Color,
    ) {
        *self
            .chunk_data
            .get_unchecked_mut(ch)
            .colors
            .get_unchecked(px)
            .get() = color;

        self.chunk_data[ch].dirty = true;
    }

    #[inline]
    unsafe fn set_color_local_unchecked(&mut self, x: i32, y: i32, col: Color) {
        self.set_color_from_index_unchecked(Self::local_to_indices(x, y), col);
    }

    #[inline]
    fn light_from_index(&self, (ch, px, ..): (usize, usize, u16, u16)) -> &[f32; 3] {
        // Safety: slicing [f32; 4] as &[f32; 3] will never fail
        unsafe {
            (*self.chunk_data[ch].lights[px].get())[0..3]
                .try_into()
                .unwrap_unchecked()
        }
    }

    #[inline]
    unsafe fn set_light_local_unchecked(&mut self, x: i32, y: i32, light: [f32; 3]) {
        self.set_light_from_index_unchecked(Self::local_to_indices(x, y), light);
    }

    #[inline]
    unsafe fn light_from_index_unchecked(
        &self,
        (ch, px, ..): (usize, usize, u16, u16),
    ) -> [f32; 3] {
        (*self
            .chunk_data
            .get_unchecked(ch)
            .lights
            .get_unchecked(px)
            .get())[0..3]
            .try_into()
            .unwrap_unchecked()
    }

    #[inline]
    fn set_light_from_index(&mut self, (ch, px, ..): (usize, usize, u16, u16), light: [f32; 3]) {
        unsafe {
            *self.chunk_data[ch].lights[px].get() = [light[0], light[1], light[2], 1.0];
        }
    }

    #[inline]
    unsafe fn set_light_from_index_unchecked(
        &mut self,
        (ch, px, ..): (usize, usize, u16, u16),
        light: [f32; 3],
    ) {
        *self
            .chunk_data
            .get_unchecked_mut(ch)
            .lights
            .get_unchecked(px)
            .get() = [light[0], light[1], light[2], 1.0];
    }

    // (chunk index, pixel index, pixel x in chunk, pixel y in chunk)
    #[inline(always)]
    fn local_to_indices(x: i32, y: i32) -> (usize, usize, u16, u16) {
        let size = i32::from(CHUNK_SIZE);
        // div_euclid is the same as div_floor in this case (div_floor is currenlty unstable)
        let rel_chunk_x = x.div_euclid(i32::from(CHUNK_SIZE)) as i8;
        let rel_chunk_y = y.div_euclid(i32::from(CHUNK_SIZE)) as i8;

        let chunk_px_x = x.rem_euclid(size) as u16;
        let chunk_px_y = y.rem_euclid(size) as u16;

        (
            (rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3,
            (chunk_px_x + chunk_px_y * CHUNK_SIZE) as usize,
            chunk_px_x,
            chunk_px_y,
        )
    }

    fn finish_dirty_rects(&mut self) {
        for i in 0..9 {
            if self.min_x[i] == CHUNK_SIZE + 1 {
                self.chunk_data[i].dirty_rect = None;
            } else {
                self.chunk_data[i].dirty_rect = Some(Rect::new_wh(
                    i32::from(self.min_x[i]),
                    i32::from(self.min_y[i]),
                    self.max_x[i] - self.min_x[i] + 1,
                    self.max_y[i] - self.min_y[i] + 1,
                ));
            }
        }
    }
}

impl SimulationHelper for SimulationHelperChunk<'_, '_> {
    #[inline]
    fn pixel_local(&self, x: i32, y: i32) -> &MaterialInstance {
        self.pixel_from_index(Self::local_to_indices(x, y))
    }

    #[inline]
    fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        self.set_pixel_from_index(Self::local_to_indices(x, y), mat);
    }

    #[inline]
    fn color_local(&self, x: i32, y: i32) -> Color {
        self.color_from_index(Self::local_to_indices(x, y))
    }

    #[inline]
    fn set_color_local(&mut self, x: i32, y: i32, col: Color) {
        self.set_color_from_index(Self::local_to_indices(x, y), col);
    }

    #[inline]
    fn set_all_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        let inds = Self::local_to_indices(x, y);
        self.set_color_from_index(inds, mat.color);
        self.set_light_from_index(inds, mat.light);
        self.set_pixel_from_index(inds, mat);
    }

    #[inline]
    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity) {
        self.particles.push(Particle::new(
            material,
            Position {
                x: pos.x + f64::from(self.chunk_x) * f64::from(CHUNK_SIZE),
                y: pos.y + f64::from(self.chunk_y) * f64::from(CHUNK_SIZE),
            },
            vel,
        ));
    }

    fn light_local(&self, x: i32, y: i32) -> &[f32; 3] {
        self.light_from_index(Self::local_to_indices(x, y))
    }

    fn set_light_local(&mut self, x: i32, y: i32, light: [f32; 3]) {
        self.set_light_from_index(Self::local_to_indices(x, y), light);
    }
}

struct SimulationHelperRigidBody<'a, C: Chunk> {
    air: MaterialInstance,
    chunk_handler: &'a mut ChunkHandler<C>,
    rigidbodies: &'a mut Vec<FSRigidBody>,
    particles: &'a mut Vec<Particle>,
    physics: &'a mut Physics,
}

impl<C: Chunk + Send> SimulationHelper for SimulationHelperRigidBody<'_, C> {
    fn pixel_local(&self, x: i32, y: i32) -> &MaterialInstance {
        let world_mat = self.chunk_handler.pixel(i64::from(x), i64::from(y)); // TODO: consider changing the args to i64
        if let Ok(m) = world_mat {
            if m.material_id != *material::AIR {
                return m;
            }
        }

        for i in 0..self.rigidbodies.len() {
            let cur = &self.rigidbodies[i];
            if let Some(body) = cur.get_body(self.physics) {
                let s = (-body.rotation().angle()).sin();
                let c = (-body.rotation().angle()).cos();

                let tx = x as f32 - body.translation().x * PHYSICS_SCALE;
                let ty = y as f32 - body.translation().y * PHYSICS_SCALE;

                let nt_x = (tx * c - ty * s) as i32;
                let nt_y = (tx * s + ty * c) as i32;

                if nt_x >= 0 && nt_y >= 0 && nt_x < cur.width.into() && nt_y < cur.width.into() {
                    let px = &cur.pixels[(nt_x + nt_y * i32::from(cur.width)) as usize];

                    if px.material_id != *material::AIR {
                        return px;
                    }
                }
            }
        }

        &self.air
    }

    fn set_pixel_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        let _ignore = self
            .chunk_handler
            .set_pixel(i64::from(x), i64::from(y), mat); // TODO: consider changing the args to i64
    }

    fn color_local(&self, x: i32, y: i32) -> Color {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(i64::from(x), i64::from(y));
        let chunk = self.chunk_handler.chunk_at((chunk_x, chunk_y));

        if let Some(ch) = chunk {
            let col_r = ch.color(
                (i64::from(x) - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                (i64::from(y) - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
            );
            if let Ok(col) = col_r {
                if col.a > 0 {
                    return col;
                }
            }
        }

        for i in 0..self.rigidbodies.len() {
            let cur = &self.rigidbodies[i];
            if let Some(body) = cur.get_body(self.physics) {
                let s = (-body.rotation().angle()).sin();
                let c = (-body.rotation().angle()).cos();

                let tx = x as f32 - body.translation().x * PHYSICS_SCALE;
                let ty = y as f32 - body.translation().y * PHYSICS_SCALE;

                let nt_x = (tx * c - ty * s) as i32;
                let nt_y = (tx * s + ty * c) as i32;

                if nt_x >= 0 && nt_y >= 0 && nt_x < cur.width.into() && nt_y < cur.width.into() {
                    let px = cur.pixels[(nt_x + nt_y * i32::from(cur.width)) as usize].clone();

                    if px.material_id != *material::AIR {
                        return px.color;
                    }
                }
            }
        }

        Color::rgba(0, 0, 0, 0)
    }

    fn set_color_local(&mut self, x: i32, y: i32, col: Color) {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(i64::from(x), i64::from(y));
        let chunk = self.chunk_handler.chunk_at_mut_dyn((chunk_x, chunk_y));

        if let Some(ch) = chunk {
            let _ignore = ch.set_color(
                (i64::from(x) - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                (i64::from(y) - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
                col,
            );
        }
    }

    #[inline]
    fn add_particle(&mut self, material: MaterialInstance, pos: Position, vel: Velocity) {
        self.particles.push(Particle::new(material, pos, vel));
    }

    fn light_local(&self, _x: i32, _y: i32) -> &[f32; 3] {
        // TODO
        &[0.0; 3]
    }

    fn set_light_local(&mut self, _x: i32, _y: i32, _light: [f32; 3]) {
        // TODO
    }

    #[inline]
    fn set_all_local(&mut self, x: i32, y: i32, mat: MaterialInstance) {
        // TODO: could be optimized
        self.set_color_local(x, y, mat.color);
        self.set_light_local(x, y, mat.light);
        self.set_pixel_local(x, y, mat);
    }
}

#[derive(Debug)]
pub struct SimulatorChunkContext<'a> {
    // using UnsafeCell to allow mutations to disjoint indices from different threads
    pub pixels: &'a [UnsafeCell<MaterialInstance>; (CHUNK_SIZE * CHUNK_SIZE) as usize],
    pub colors: &'a [UnsafeCell<Color>; (CHUNK_SIZE * CHUNK_SIZE) as usize],
    pub lights: &'a [UnsafeCell<[f32; 4]>; CHUNK_SIZE as usize * CHUNK_SIZE as usize],
    pub dirty: bool,
    pub dirty_rect: Option<Rect<i32>>,
}
unsafe impl<'a> Send for SimulatorChunkContext<'a> {}
unsafe impl<'a> Sync for SimulatorChunkContext<'a> {}

impl Simulator {
    #[warn(clippy::too_many_arguments)]
    #[profiling::function]
    pub fn simulate_chunk(
        chunk_x: i32,
        chunk_y: i32,
        chunk_data: &mut [SimulatorChunkContext; 9],
        particles: &mut Vec<Particle>,
        registries: Arc<Registries>,
    ) {
        const CENTER_CHUNK: usize = 4;

        let my_dirty_rect_o = chunk_data[CENTER_CHUNK].dirty_rect;
        if my_dirty_rect_o.is_none() {
            for d in chunk_data {
                d.dirty_rect = None;
            }
            return;
        }
        let my_dirty_rect = my_dirty_rect_o.unwrap();

        let mut helper = SimulationHelperChunk {
            chunk_data,
            min_x: [CHUNK_SIZE + 1; 9],
            min_y: [CHUNK_SIZE + 1; 9],
            max_x: [0; 9],
            max_y: [0; 9],
            particles,
            chunk_x,
            chunk_y,
        };

        let rng = fastrand::Rng::new();
        {
            /// `x` and `y` MUST be in `0..CHUNK_SIZE` (unchecked)
            // this being inlined is important for performance
            #[inline(always)]
            fn process(
                x: i32,
                y: i32,
                helper: &mut SimulationHelperChunk,
                rng: &Rng,
                _registries: &Registries,
            ) {
                // Safety: x and y are assumed to be within the chunk

                // no real performance benefit so it probably figures this out from the other `unchecked` calls
                // if x < 0 || x >= i32::from(CHUNK_SIZE) || y < 0 || y >= i32::from(CHUNK_SIZE) {
                //     unsafe { std::hint::unreachable_unchecked() }
                // }

                let cur = unsafe { helper.pixel_local_unchecked(x, y) }.clone();

                if let Some(mat) = Simulator::simulate_pixel(x, y, &cur, helper, rng) {
                    unsafe {
                        helper.set_color_local_unchecked(x, y, mat.color);
                        helper.set_light_local_unchecked(x, y, mat.light);
                        helper.set_pixel_local_unchecked(x, y, mat);
                    }
                }
            }

            profiling::scope!("loop");
            if rng.bool() {
                for y in my_dirty_rect.range_tb().rev() {
                    for x in my_dirty_rect.range_lr() {
                        // Safety: dirty rects are always within the chunk
                        process(x, y, &mut helper, &rng, &registries);
                    }
                }
            } else {
                for y in my_dirty_rect.range_tb().rev() {
                    for x in my_dirty_rect.range_lr().rev() {
                        // Safety: dirty rects are always within the chunk
                        process(x, y, &mut helper, &rng, &registries);
                    }
                }
            }
        }

        helper.finish_dirty_rects();
    }

    #[allow(clippy::unnecessary_unwrap)]
    #[allow(clippy::needless_range_loop)]
    #[profiling::function]
    pub fn simulate_rigidbodies<C: Chunk + Send>(
        chunk_handler: &mut ChunkHandler<C>,
        rigidbodies: &mut Vec<FSRigidBody>,
        physics: &mut Physics,
        particles: &mut Vec<Particle>,
    ) {
        let mut dirty = vec![false; rigidbodies.len()];
        let mut needs_remesh = vec![false; rigidbodies.len()];
        for i in 0..rigidbodies.len() {
            let rb_w = rigidbodies[i].width;
            let rb_h = rigidbodies[i].height;
            let body_opt = rigidbodies[i].get_body(physics);

            if let Some(body) = body_opt {
                let s = body.rotation().angle().sin();
                let c = body.rotation().angle().cos();
                let pos_x = body.translation().x * PHYSICS_SCALE;
                let pos_y = body.translation().y * PHYSICS_SCALE;

                let mut helper = SimulationHelperRigidBody {
                    air: MaterialInstance::air(),
                    chunk_handler,
                    rigidbodies,
                    particles,
                    physics,
                };

                let rng = fastrand::Rng::new();
                for rb_y in 0..rb_w {
                    for rb_x in 0..rb_h {
                        let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                        let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                        // let cur = helper.get_pixel_local(tx as i32, ty as i32);
                        let cur =
                            helper.rigidbodies[i].pixels[(rb_x + rb_y * rb_w) as usize].clone();

                        let res =
                            Self::simulate_pixel(tx as i32, ty as i32, &cur, &mut helper, &rng);

                        if let Some(mat) = res {
                            helper.rigidbodies[i].pixels[(rb_x + rb_y * rb_w) as usize] =
                                mat.clone();
                            dirty[i] = true;
                            if (cur.physics == PhysicsType::Solid
                                && mat.physics != PhysicsType::Solid)
                                || (cur.physics != PhysicsType::Solid
                                    && mat.physics == PhysicsType::Solid)
                            {
                                needs_remesh[i] = true;
                            }
                        }
                    }
                }
            }
        }

        for i in 0..rigidbodies.len() {
            if dirty[i] && !needs_remesh[i] {
                // don't bother updating the image if it's going to be destroyed anyway
                rigidbodies[i].image_dirty = true;
            }
        }

        let mut new_rb: Vec<FSRigidBody> = rigidbodies
            .drain(..)
            .enumerate()
            .flat_map(|(i, mut rb): (usize, FSRigidBody)| {
                if needs_remesh[i] {
                    let pos = (
                        rb.get_body(physics).unwrap().translation().x,
                        rb.get_body(physics).unwrap().translation().y,
                    );

                    let rb_pos = *rb.get_body(physics).unwrap().translation();
                    let rb_angle = rb.get_body(physics).unwrap().rotation().angle();
                    let rb_linear_velocity = *rb.get_body(physics).unwrap().linvel();
                    let rb_angular_velocity = rb.get_body(physics).unwrap().angvel();

                    physics.bodies.remove(
                        rb.body.take().unwrap(),
                        &mut physics.islands,
                        &mut physics.colliders,
                        &mut physics.impulse_joints,
                        &mut physics.multibody_joints,
                        true,
                    );
                    let mut r = rigidbody::FSRigidBody::make_bodies(
                        &rb.pixels, rb.width, rb.height, physics, pos,
                    )
                    .unwrap_or_default();

                    for rb in &mut r {
                        rb.get_body_mut(physics)
                            .unwrap()
                            .set_position(Isometry2::new(rb_pos, rb_angle), true);
                        rb.get_body_mut(physics)
                            .unwrap()
                            .set_linvel(rb_linear_velocity, true);
                        rb.get_body_mut(physics)
                            .unwrap()
                            .set_angvel(rb_angular_velocity, true);
                    }

                    r
                } else {
                    vec![rb]
                }
            })
            .collect();

        rigidbodies.append(&mut new_rb);
    }

    #[allow(clippy::inline_always)]
    #[inline(always)] // speeds up simulate_chunk by ~35%
    fn simulate_pixel(
        x: i32,
        y: i32,
        cur: &MaterialInstance,
        helper: &mut impl SimulationHelper,
        rng: &fastrand::Rng,
    ) -> Option<MaterialInstance> {
        let mut new_mat = None;

        #[allow(clippy::single_match)]
        match cur.physics {
            PhysicsType::Sand => {
                let can_move_down = helper.pixel_local(x, y + 1).physics == PhysicsType::Air;
                let can_move_down_left =
                    helper.pixel_local(x - 1, y + 1).physics == PhysicsType::Air;
                let can_move_down_right =
                    helper.pixel_local(x + 1, y + 1).physics == PhysicsType::Air;

                let can_move_dl_or_dr = can_move_down_right || can_move_down_left;

                if can_move_down && (!can_move_dl_or_dr || rng.f32() > 0.1) {
                    // are a few pixels below clear
                    let empty_below = (0..4).all(|i| {
                        // don't include self or one below
                        helper.pixel_local(x, y + i + 2).physics == PhysicsType::Air
                    });

                    if empty_below {
                        // if a few pixels below are clear, become a particle
                        helper.add_particle(
                            cur.clone(),
                            Position { x: f64::from(x), y: f64::from(y) },
                            Velocity { x: (rng.f64() - 0.5) * 0.5, y: 1.0 + rng.f64() },
                        );
                    } else {
                        // otherwise move 1 or 2 pixels down
                        if rng.bool() && helper.pixel_local(x, y + 2).physics == PhysicsType::Air {
                            helper.set_all_local(x, y + 2, cur.clone());
                        } else {
                            helper.set_all_local(x, y + 1, cur.clone());
                        }
                    }

                    new_mat = Some(MaterialInstance::air());
                } else {
                    // !can_move_down && can_move_dl_or_dr

                    let above_is_air = helper.pixel_local(x, y - 1).physics == PhysicsType::Air;

                    // covered pixels are less likely to move down to the sides
                    if above_is_air || rng.f32() > 0.5 {
                        if can_move_down_left && can_move_down_right {
                            // randomly pick a direction
                            if rng.bool() {
                                helper.set_all_local(x + 1, y + 1, cur.clone());
                            } else {
                                helper.set_all_local(x - 1, y + 1, cur.clone());
                            }
                            new_mat = Some(MaterialInstance::air());
                        } else if can_move_down_left {
                            // chance to move by 2
                            if rng.bool()
                                && helper.pixel_local(x - 2, y + 1).physics == PhysicsType::Air
                                && helper.pixel_local(x - 2, y + 2).physics != PhysicsType::Air
                            {
                                helper.set_all_local(x - 2, y + 1, cur.clone());
                                new_mat = Some(MaterialInstance::air());
                            } else {
                                helper.set_all_local(x - 1, y + 1, cur.clone());
                                new_mat = Some(MaterialInstance::air());
                            }
                        } else if can_move_down_right {
                            // chance to move by 2
                            if rng.bool()
                                && helper.pixel_local(x + 2, y + 1).physics == PhysicsType::Air
                                && helper.pixel_local(x + 2, y + 2).physics != PhysicsType::Air
                            {
                                helper.set_all_local(x + 2, y + 1, cur.clone());
                                new_mat = Some(MaterialInstance::air());
                            } else {
                                helper.set_all_local(x + 1, y + 1, cur.clone());
                                new_mat = Some(MaterialInstance::air());
                            }
                        }
                    }
                }
            },
            _ => {},
        }

        new_mat
    }
}
