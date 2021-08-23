use specs::{Component, Entities, Join, System, Write, WriteStorage, storage::BTreeStorage};
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

        let (entities, mut pos, mut vel, mut game_ent, mut phys_ent, mut persistent, mut hitbox) = data;
        // let chunk_handler = chunk_handler.unwrap().0;
        let chunk_handler = &mut *self.chunk_handler;

        let debug_visualize = false;

        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut pos, &mut vel, &mut game_ent, &mut phys_ent, persistent.maybe(), &mut hitbox).join().for_each(|(ent, pos, vel, game_ent, phys_ent, persistent, hitbox): (specs::Entity, &mut Position, &mut Velocity, &mut GameEntity, &mut PhysicsEntity, Option<&Persistent>, &mut Hitbox)| {
            // profiling::scope!("Particle");

            let lx = pos.x;
            let ly = pos.y;

            let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(lx as i64, ly as i64);
            // skip if chunk not active
            if persistent.is_none() && !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                return;
            }

            vel.y += 0.1;

            let dx = vel.x;
            let dy = vel.y;

            let mut new_pos_x = lx;
            let mut new_pos_y = ly;

            let steps = ((dx.abs() + dy.abs()) as u32 + 1).max(2);
            for s in 0..steps {
                // profiling::scope!("step");
                let thru = f64::from(s + 1) / f64::from(steps);

                new_pos_x += dx / steps as f64;
                new_pos_y += dy / steps as f64;

                let mut collided_x = false;
                let mut collided_y = false;

                let steps_x = ((hitbox.x2 - hitbox.x1).signum() * (hitbox.x2 - hitbox.x1).abs().ceil()) as u16;
                let steps_y = ((hitbox.y2 - hitbox.y1).signum() * (hitbox.y2 - hitbox.y1).abs().ceil()) as u16;
                
                let edge_clip_distance = 2.0;

                for h_dx in 0..=steps_x {
                    let h_dx = (h_dx as f32 / steps_x as f32) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                    for h_dy in 0..=steps_y {
                        let h_dy = (h_dy as f32 / steps_y as f32) * (hitbox.y2 - hitbox.y1) + hitbox.y1;

                        if let Ok(mat) = chunk_handler.get((new_pos_x + h_dx as f64).floor() as i64, (pos.y + h_dy as f64).floor() as i64) {
                            if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                if h_dy - hitbox.y1 < edge_clip_distance {
                                    let clip_y = ((pos.y + h_dy as f64).floor() + 1.0) - (pos.y + hitbox.y1 as f64) + 0.05;
                                    // log::debug!("clip_y = {}", clip_y);
                                    let mut would_clip_collide = false;
                                    'clip_collide: for h_dx in 0..=steps_x {
                                        let h_dx = (h_dx as f32 / steps_x as f32) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                                        for h_dy in 0..=steps_y {
                                            let h_dy = (h_dy as f32 / steps_y as f32) * (hitbox.y2 - hitbox.y1) + hitbox.y1;
                    
                                            if let Ok(mat) = chunk_handler.get((new_pos_x + h_dx as f64).floor() as i64, (pos.y + clip_y + h_dy as f64).floor() as i64) {
                                                if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                                    would_clip_collide = true;
                                                    break 'clip_collide;
                                                }
                                            }
                                        }
                                    }

                                    if would_clip_collide {
                                        collided_x = true;
                                        if debug_visualize {
                                            let _ignore = chunk_handler.set((new_pos_x + h_dx as f64).floor() as i64, (pos.y + clip_y + h_dy as f64).floor() as i64, MaterialInstance {
                                                color: sdl2::pixels::Color::RGB(255, 255, 0),
                                                ..*mat
                                            });
                                        }
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
                                    let clip_y = (pos.y + h_dy as f64).floor() - (pos.y + hitbox.y2 as f64) - 0.05;
                                    // log::debug!("clip_y = {}", clip_y);
                                    let mut would_clip_collide = false;
                                    'clip_collide: for h_dx in 0..=steps_x {
                                        let h_dx = (h_dx as f32 / steps_x as f32) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                                        for h_dy in 0..=steps_y {
                                            let h_dy = (h_dy as f32 / steps_y as f32) * (hitbox.y2 - hitbox.y1) + hitbox.y1;
                    
                                            if let Ok(mat) = chunk_handler.get((new_pos_x + h_dx as f64).floor() as i64, (pos.y + clip_y + h_dy as f64).floor() as i64) {
                                                if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                                    would_clip_collide = true;
                                                    if debug_visualize {
                                                        let _ignore = chunk_handler.set((new_pos_x + h_dx as f64).floor() as i64, (pos.y + clip_y + h_dy as f64).floor() as i64, MaterialInstance {
                                                            color: sdl2::pixels::Color::RGB(127, 127, 0),
                                                            ..*mat
                                                        });
                                                    }
                                                    break 'clip_collide;
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
                                        let _ignore = chunk_handler.set((new_pos_x + h_dx as f64).floor() as i64, (pos.y + h_dy as f64).floor() as i64, MaterialInstance {
                                            color: sdl2::pixels::Color::RGB(255, 255, 0),
                                            ..*mat
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
                    let h_dx = (h_dx as f32 / steps_x as f32) * (hitbox.x2 - hitbox.x1) + hitbox.x1;
                    for h_dy in 0..=steps_y {
                        let h_dy = (h_dy as f32 / steps_y as f32) * (hitbox.y2 - hitbox.y1) + hitbox.y1;

                        if let Ok(mat) = chunk_handler.get((pos.x + h_dx as f64).floor() as i64, (new_pos_y + h_dy as f64).floor() as i64) {
                            if mat.physics == PhysicsType::Solid || mat.physics == PhysicsType::Sand {
                                collided_y = true;
                                if debug_visualize {
                                    let _ignore = chunk_handler.set((pos.x + h_dx as f64).floor() as i64, (new_pos_y + h_dy as f64).floor() as i64, MaterialInstance {
                                        color: sdl2::pixels::Color::RGB(255, 0, 255),
                                        ..*mat
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