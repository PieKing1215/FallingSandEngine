use rand::Rng;
use serde::{Deserialize, Serialize};
use specs::{storage::BTreeStorage, Component, Entities, Join, System, Write, WriteStorage};

mod player;
pub use player::*;

use crate::game::common::world::{
    material::{Color, MaterialInstance, PhysicsType},
    pixel_to_chunk_pos,
};

use super::{
    particle::{Particle, ParticleSystem},
    ChunkHandlerGeneric, ChunkState, Position, Velocity,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntity;

impl Component for GameEntity {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hitbox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Component for Hitbox {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsEntity {
    pub gravity: f64,
    pub on_ground: bool,
    pub edge_clip_distance: f32,
    pub collision: bool,
    pub collide_with_sand: bool,
}

impl Component for PhysicsEntity {
    type Storage = BTreeStorage<Self>;
}

// TODO: this struct sucks, add more detailed info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionDetector {
    pub collided: bool,
}

impl Component for CollisionDetector {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persistent;

impl Component for Persistent {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdatePhysicsEntities<'a, H: ChunkHandlerGeneric> {
    pub chunk_handler: &'a mut H,
}

impl<'a, H: ChunkHandlerGeneric> UpdatePhysicsEntities<'a, H> {
    fn check_collide(&self, x: i64, y: i64, phys_ent: &PhysicsEntity) -> Option<&MaterialInstance> {
        self.chunk_handler.get(x, y).ok().filter(|mat| {
            mat.physics == PhysicsType::Solid
                || (mat.physics == PhysicsType::Sand && phys_ent.collide_with_sand)
        })
    }
}

impl<'a, H: ChunkHandlerGeneric> System<'a> for UpdatePhysicsEntities<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, GameEntity>,
        WriteStorage<'a, PhysicsEntity>,
        WriteStorage<'a, Persistent>,
        WriteStorage<'a, Hitbox>,
        WriteStorage<'a, CollisionDetector>,
        Write<'a, ParticleSystem>,
    );

    #[allow(clippy::too_many_lines)]
    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdatePhysicsEntities::run");

        let (
            entities,
            mut pos,
            mut vel,
            mut game_ent,
            mut phys_ent,
            persistent,
            mut hitbox,
            mut collision_detect,
            mut particle_system,
        ) = data;

        let debug_visualize = false;

        let mut create_particles: Vec<Particle> = vec![];

        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut pos, &mut vel, &mut game_ent, &mut phys_ent, persistent.maybe(), &mut hitbox, (&mut collision_detect).maybe()).join().for_each(|(_ent, pos, vel, _game_ent, phys_ent, persistent, hitbox, mut collision_detect): (specs::Entity, &mut Position, &mut Velocity, &mut GameEntity, &mut PhysicsEntity, Option<&Persistent>, &mut Hitbox, Option<&mut CollisionDetector>)| {

            // skip if chunk not active
            let (chunk_x, chunk_y) = pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
            if persistent.is_none() && !matches!(self.chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                return;
            }

            phys_ent.on_ground = false;

            // skip if no collide
            if !phys_ent.collision {
                pos.x += vel.x;
                pos.y += vel.y;
                return;
            }

            // cache coordinates for every point in the hitbox
            // (basically instead of `for x in 0..w { for y in 0..h {}}` you can do `for (x,y) in r`)
            // this helps reduce code duplication because normally every use of this would have to be two nested loops

            let steps_x = ((hitbox.x2 - hitbox.x1).signum() * (hitbox.x2 - hitbox.x1).abs().ceil()) as u16;
            let steps_y = ((hitbox.y2 - hitbox.y1).signum() * (hitbox.y2 - hitbox.y1).abs().ceil()) as u16;

            let r: Vec<(f32, f32)> = (0..=steps_x).flat_map(move |a| (0..=steps_y).map(move |b| (a, b))).map(|(xs, ys)| {
                ((f32::from(xs) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1,
                (f32::from(ys) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1)
            }).collect();

            // if currently intersected, try to get out

            let mut n_intersect = 0;
            let mut avg_in_x = 0.0;
            let mut avg_in_y = 0.0;

            for &(h_dx, h_dy) in &r {
                if self.check_collide((pos.x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64, phys_ent).is_some() {
                    n_intersect += 1;
                    avg_in_x += h_dx;
                    avg_in_y += h_dy;
                }
            }

            if n_intersect > 0 {
                pos.x += f64::from(if avg_in_x == 0.0 { 0.0 } else { -avg_in_x.signum() });
                pos.y += f64::from(if avg_in_y == 0.0 { 0.0 } else { -avg_in_y.signum() });
            }

            // do collision detection
            //   split into a number of steps
            //     each step moves x and y separately so we know which velocities to cancel

            vel.y += phys_ent.gravity;

            let dx = vel.x;
            let dy = vel.y;

            let mut new_pos_x = pos.x;
            let mut new_pos_y = pos.y;

            let steps = ((dx.abs() + dy.abs()) as u32 + 1).max(3);
            for _ in 0..steps {

                new_pos_x += dx / f64::from(steps);
                new_pos_y += dy / f64::from(steps);

                // check x motion

                let mut collided_x = false;
                for &(h_dx, h_dy) in &r {
                    if let Some(mat) = self.check_collide((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64, phys_ent).copied() {

                        let clip_ceil = (h_dy - hitbox.y1 < phys_ent.edge_clip_distance).then(|| ((pos.y + f64::from(h_dy)).floor() + 1.0) - (pos.y + f64::from(hitbox.y1)) + 0.05);
                        let clip_floor = (hitbox.y2 - h_dy < phys_ent.edge_clip_distance).then(|| (pos.y + f64::from(h_dy)).floor() - (pos.y + f64::from(hitbox.y2)) - 0.05);

                        if let Some(clip_y) = clip_ceil.or(clip_floor) {
                            let mut would_clip_collide = false;
                            for &(h_dx, h_dy) in &r {
                                if let Some(mat) = self.check_collide((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64, phys_ent).copied() {
                                    would_clip_collide = true;
                                    if debug_visualize {
                                        let _ignore = self.chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                            color: Color::rgb(255, 255, 0),
                                            ..mat
                                        });
                                    }
                                    break;
                                }
                            }

                            if would_clip_collide {
                                collided_x = true;
                            } else {
                                new_pos_y += clip_y;
                                pos.y += clip_y;

                                // larger step means more slowdown
                                // 1.0 -> 0.988
                                // 2.0 -> 0.8
                                // 2.5 -> 0.515
                                // 3.0 -> 0.5 (clamped)
                                vel.x *= (1.0 - (clip_y.abs() / 3.0).powi(4)).clamp(0.5, 1.0);
                            }
                        } else if mat.physics == PhysicsType::Sand && self.chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64, MaterialInstance::air()).is_ok() {
                            create_particles.push(
                                Particle::new(
                                    mat,
                                    Position { x: (new_pos_x + f64::from(h_dx)).floor(), y: (pos.y + f64::from(h_dy)).floor().floor() },
                                    Velocity { x: rand::thread_rng().gen_range(-0.5..=0.5) + 2.0 * vel.x.signum(), y: rand::thread_rng().gen_range(-0.5..=0.5)},
                                )
                            );

                            vel.x *= 0.99;
                        }else {
                            collided_x = true;
                            if debug_visualize {
                                let _ignore = self.chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                    color: Color::rgb(255, 255, 0),
                                    ..mat
                                });
                            }
                        }
                    }
                }

                if collided_x {
                    vel.x = if vel.x.abs() > 0.25 { vel.x * 0.5 } else { 0.0 };
                    if let Some(c) = &mut collision_detect {
                        c.collided = true;
                    }
                } else {
                    pos.x = new_pos_x;
                }

                // check y motion

                let mut collided_y = false;
                for &(h_dx, h_dy) in &r {
                    if let Some(mat) = self.check_collide((pos.x + f64::from(h_dx)).floor() as i64, (new_pos_y + f64::from(h_dy)).floor() as i64, phys_ent).copied() {
                        if (vel.y < -0.001 || vel.y > 1.0) && mat.physics == PhysicsType::Sand && self.chunk_handler.set((pos.x + f64::from(h_dx)).floor() as i64, (new_pos_y + f64::from(h_dy)).floor() as i64, MaterialInstance::air()).is_ok() {
                            create_particles.push(
                                Particle::new(
                                    mat,
                                    Position { x: (pos.x + f64::from(h_dx)).floor(), y: (new_pos_y + f64::from(h_dy)).floor() },
                                    Velocity { x: rand::thread_rng().gen_range(-0.5..=0.5), y: rand::thread_rng().gen_range(-1.0..=0.0)},
                                )
                            );

                            if vel.y > 0.0 {
                                vel.y *= 0.9;
                            }

                            vel.y *= 0.99;
                        } else {
                            collided_y = true;
                            if debug_visualize {
                                let _ignore = self.chunk_handler.set((pos.x + f64::from(h_dx)).floor() as i64, (new_pos_y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                    color: Color::rgb(255, 0, 255),
                                    ..mat
                                });
                            }
                            break;
                        }
                    }
                }

                if collided_y {
                    vel.x *= 0.96;

                    if dy > 0.0 {
                        phys_ent.on_ground = true;
                    }

                    vel.y = if vel.y.abs() > 0.5 { vel.y * 0.75 } else { 0.0 };

                    if let Some(c) = &mut collision_detect {
                        c.collided = true;
                    }
                } else {
                    pos.y = new_pos_y;
                }
            }
        });

        for part in create_particles {
            particle_system.active.push(part);
        }
    }
}
