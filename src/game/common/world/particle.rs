use std::marker::PhantomData;

use crate::game::common::world::{ChunkState, ecs::ChunkHandlerResource, material::{PhysicsType, TEST_MATERIAL}};
use super::{Chunk, ChunkHandlerGeneric, Position, Velocity, gen::WorldGenerator, material::MaterialInstance};

use sdl2::pixels::Color;
use serde::{Serialize, Deserialize};
use specs::{Component, Entities, Join, ParJoin, Read, ReadExpect, System, Write, WriteExpect, WriteStorage, prelude::ParallelIterator, storage::BTreeStorage};

// #[derive(Serialize, Deserialize)]
// pub struct Particle {
//     pub material: MaterialInstance,
//     pub x: f32,
//     pub y: f32,
//     pub vx: f32,
//     pub vy: f32,
//     pub in_object_state: InObjectState,
// }

// impl Particle {
//     pub fn new(material: MaterialInstance, x: f32, y: f32, vx: f32, vy: f32) -> Self {
//         Self {
//             material,
//             x, y,
//             vx, vy,
//             in_object_state: InObjectState::FirstFrame,
//         }
//     }
// }

#[derive(Debug)]
pub struct Particle {
    pub material: MaterialInstance,
    in_object_state: InObjectState,
}

impl Particle {
    pub fn of(material: MaterialInstance) -> Self {
        Self {
            material,
            in_object_state: InObjectState::FirstFrame,
        }
    }
}

impl Component for Particle {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum InObjectState {
    FirstFrame,
    Inside,
    Outside,
}

pub struct UpdateParticles<'a>{
    pub chunk_handler: &'a mut (dyn ChunkHandlerGeneric)
}

impl<'a> System<'a> for UpdateParticles<'a> {
    type SystemData = (Entities<'a>,
                       WriteStorage<'a, Particle>,
                       WriteStorage<'a, Position>,
                       WriteStorage<'a, Velocity>);

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateParticles::run");

        let (entities, mut particle, mut pos, mut vel) = data;
        // let chunk_handler = chunk_handler.unwrap().0;
        let chunk_handler = &mut self.chunk_handler;

        let new_p = entities.create();
        particle.insert(new_p, Particle {
            material: MaterialInstance {
                material_id: TEST_MATERIAL.id,
                physics: PhysicsType::Sand,
                color: Color::RGB(64, 255, 255),
            },
            in_object_state: InObjectState::FirstFrame,
        }).expect("Failed to insert Particle");
        pos.insert(new_p, Position{x: (rand::random::<f32>() - 0.5) * 10.0, y: -100.0}).expect("Failed to insert Position");
        vel.insert(new_p, Velocity{x: (rand::random::<f32>() - 0.5) * 4.0, y: (rand::random::<f32>() - 0.75) * 2.0}).expect("Failed to insert Velocity");
        
        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut particle, &mut pos, &mut vel).join().for_each(|(ent, part, pos, vel)| {
            // profiling::scope!("Particle");

            let lx = pos.x;
            let ly = pos.y;

            let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(lx as i64, ly as i64);
            // skip if chunk not active
            if !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                return;
            }

            vel.y += 0.1;

            let dx = vel.x;
            let dy = vel.y;

            let steps = (dx.abs() + dy.abs()).sqrt() as u32 + 1;
            for s in 0..steps {
                // profiling::scope!("step");
                let thru = (s + 1) as f32 / steps as f32;

                pos.x = lx + dx * thru;
                pos.y = ly + dy * thru;

                if let Ok(mat) = chunk_handler.get(pos.x as i64, pos.y as i64) {
                    if mat.physics == PhysicsType::Air {
                        part.in_object_state = InObjectState::Outside;
                    }else{
                        let is_object = mat.physics == PhysicsType::Object;

                        match part.in_object_state {
                            InObjectState::FirstFrame => {
                                if is_object {
                                    part.in_object_state = InObjectState::Inside;
                                }else {
                                    part.in_object_state = InObjectState::Outside;
                                }
                            },
                            InObjectState::Inside => {
                                if !is_object {
                                    part.in_object_state = InObjectState::Outside;
                                }
                            },
                            InObjectState::Outside => {},
                        }

                        if !is_object || part.in_object_state == InObjectState::Outside {

                            match chunk_handler.get(lx as i64, ly as i64) {
                                Ok(m) if m.physics != PhysicsType::Air => {
                                    
                                    let succeeded = chunk_handler.displace(pos.x as i64, pos.y as i64, part.material);

                                    if succeeded {
                                        entities.delete(ent).expect("Failed to delete particle");
                                    }else{
                                        // upwarp if completely blocked
                                        vel.y = -1.0;
                                        pos.y -= 16.0;
                                    }
                                    
                                    break;
                                },
                                _ => {
                                    if chunk_handler.set(lx as i64, ly as i64, part.material).is_ok() {
                                        entities.delete(ent).expect("Failed to delete particle");
                                        break;
                                    }
                                },
                            }
                        }
                    }
                }
            }
        });
    }
}