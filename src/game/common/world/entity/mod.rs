use specs::{Component, Entities, Join, System, WriteStorage, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

mod player;
pub use player::*;

use crate::game::common::world::material::{MaterialInstance, PhysicsType};

use super::{ChunkState, ChunkHandlerGeneric, Position, Velocity};

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
pub struct PhysicsEntity;

impl Component for PhysicsEntity {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persistent;

impl Component for Persistent {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdatePhysicsEntities<'a>{
    pub chunk_handler: &'a mut (dyn ChunkHandlerGeneric)
}

impl<'a> System<'a> for UpdatePhysicsEntities<'a> {
    #[allow(clippy::type_complexity)]
    type SystemData = (Entities<'a>,
                       WriteStorage<'a, Position>,
                       WriteStorage<'a, Velocity>,
                       WriteStorage<'a, GameEntity>,
                       WriteStorage<'a, PhysicsEntity>,
                       WriteStorage<'a, Persistent>,
                       WriteStorage<'a, Hitbox>);

    #[allow(clippy::too_many_lines)]
    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdatePhysicsEntities::run");

        let (entities, mut pos, mut vel, mut game_ent, mut phys_ent, persistent, mut hitbox) = data;
        // let chunk_handler = chunk_handler.unwrap().0;
        let chunk_handler = &mut *self.chunk_handler;

        let debug_visualize = false;

        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut pos, &mut vel, &mut game_ent, &mut phys_ent, persistent.maybe(), &mut hitbox).join().for_each(|(_ent, pos, vel, _game_ent, _phys_ent, persistent, hitbox): (specs::Entity, &mut Position, &mut Velocity, &mut GameEntity, &mut PhysicsEntity, Option<&Persistent>, &mut Hitbox)| {
            // profiling::scope!("Particle");

            let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
            // skip if chunk not active
            if persistent.is_none() && !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                return;
            }

            let steps_x = ((hitbox.x2 - hitbox.x1).signum() * (hitbox.x2 - hitbox.x1).abs().ceil()) as u16;
            let steps_y = ((hitbox.y2 - hitbox.y1).signum() * (hitbox.y2 - hitbox.y1).abs().ceil()) as u16;

            // if currently intersected, try to get out

            let mut n_intersect = 0;
            let mut avg_in_x = 0.0;
            let mut avg_in_y = 0.0;

            for h_dx in 0..=steps_x {
                let h_dx = (f32::from(h_dx) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                for h_dy in 0..=steps_y {
                    let h_dy = (f32::from(h_dy) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1;

                    if let Ok(mat) = chunk_handler.get((pos.x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64) {
                        if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                            n_intersect += 1;
                            avg_in_x += h_dx;
                            avg_in_y += h_dy;
                        }
                    }
                }
            }

            if n_intersect > 0 {
                pos.x += f64::from(if avg_in_x == 0.0 { 0.0 } else { -avg_in_x.signum() });
                pos.y += f64::from(if avg_in_y == 0.0 { 0.0 } else { -avg_in_y.signum() });
            }

            vel.y += 0.1;

            let dx = vel.x;
            let dy = vel.y;

            let mut new_pos_x = pos.x;
            let mut new_pos_y = pos.y;

            let steps = ((dx.abs() + dy.abs()) as u32 + 1).max(2);
            for _ in 0..steps {
                // profiling::scope!("step");

                new_pos_x += dx / f64::from(steps);
                new_pos_y += dy / f64::from(steps);

                let mut collided_x = false;
                let mut collided_y = false;

                let edge_clip_distance = 2.0;

                for h_dx in 0..=steps_x {
                    let h_dx = (f32::from(h_dx) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                    for h_dy in 0..=steps_y {
                        let h_dy = (f32::from(h_dy) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1;

                        if let Ok(mat) = chunk_handler.get((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64).map(|m| *m) {
                            if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                if h_dy - hitbox.y1 < edge_clip_distance {
                                    let clip_y = ((pos.y + f64::from(h_dy)).floor() + 1.0) - (pos.y + f64::from(hitbox.y1)) + 0.05;
                                    // log::debug!("clip_y = {}", clip_y);
                                    let mut would_clip_collide = false;
                                    'clip_collide_a: for h_dx in 0..=steps_x {
                                        let h_dx = (f32::from(h_dx) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                                        for h_dy in 0..=steps_y {
                                            let h_dy = (f32::from(h_dy) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1;
                    
                                            if let Ok(mat) = chunk_handler.get((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64).map(|m| *m) {
                                                if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                                    would_clip_collide = true;
                                                    if debug_visualize {
                                                        let _ignore = chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                                            color: sdl2::pixels::Color::RGB(255, 255, 0),
                                                            ..mat
                                                        });
                                                    }
                                                    break 'clip_collide_a;
                                                }
                                            }
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
                                }else if hitbox.y2 - h_dy < edge_clip_distance {
                                    let clip_y = (pos.y + f64::from(h_dy)).floor() - (pos.y + f64::from(hitbox.y2)) - 0.05;
                                    // log::debug!("clip_y = {}", clip_y);
                                    let mut would_clip_collide = false;
                                    'clip_collide_b: for h_dx in 0..=steps_x {
                                        let h_dx = (f32::from(h_dx) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                                        for h_dy in 0..=steps_y {
                                            let h_dy = (f32::from(h_dy) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1;
                    
                                            if let Ok(mat) = chunk_handler.get((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64).map(|m| *m) {
                                                if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                                    would_clip_collide = true;
                                                    if debug_visualize {
                                                        let _ignore = chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + clip_y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                                            color: sdl2::pixels::Color::RGB(127, 127, 0),
                                                            ..mat
                                                        });
                                                    }
                                                    break 'clip_collide_b;
                                                }
                                            }
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
                                } else {
                                    collided_x = true;
                                    if debug_visualize {
                                        let _ignore = chunk_handler.set((new_pos_x + f64::from(h_dx)).floor() as i64, (pos.y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                            color: sdl2::pixels::Color::RGB(255, 255, 0),
                                            ..mat
                                        });
                                    }
                                    // break 'collision;
                                }
                            }
                        }
                    }
                }

                if collided_x {
                    vel.x = if vel.x.abs() > 0.25 { vel.x * 0.5 } else { 0.0 };
                } else {
                    pos.x = new_pos_x;
                }

                'collision: for h_dx in 0..=steps_x {
                    let h_dx = (f32::from(h_dx) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                    for h_dy in 0..=steps_y {
                        let h_dy = (f32::from(h_dy) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1) + hitbox.y1;

                        if let Ok(mat) = chunk_handler.get((pos.x + f64::from(h_dx)).floor() as i64, (new_pos_y + f64::from(h_dy)).floor() as i64).map(|m| *m) {
                            if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                collided_y = true;
                                if debug_visualize {
                                    let _ignore = chunk_handler.set((pos.x + f64::from(h_dx)).floor() as i64, (new_pos_y + f64::from(h_dy)).floor() as i64, MaterialInstance {
                                        color: sdl2::pixels::Color::RGB(255, 0, 255),
                                        ..mat
                                    });
                                }
                                break 'collision;
                            }
                        }
                    }
                }

                if collided_y {
                    vel.x *= 0.95;

                    vel.y = if vel.y.abs() > 0.25 { vel.y * 0.5 } else { 0.0 };
                } else {
                    pos.y = new_pos_y;
                }
            }
        });
    }
}