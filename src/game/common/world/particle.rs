use super::{
    entity::Hitbox, gen::WorldGenerator, material::MaterialInstance, Chunk, ChunkHandler,
    ChunkHandlerGeneric, FilePersistent, Position, TickTime, Velocity,
};
use crate::game::common::world::{
    material::{PhysicsType, TEST_MATERIAL},
    ChunkState,
};
use specs::{prelude::ParallelIterator, ParJoin};

use rand::prelude::Distribution;
use sdl2::pixels::Color;
use serde::{Deserialize, Serialize};
use specs::{
    saveload::{MarkerAllocator, SimpleMarker, SimpleMarkerAllocator},
    Component, Entities, Join, LazyUpdate, NullStorage, Read, ReadStorage, System, VecStorage,
    Write, WriteStorage,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    type Storage = VecStorage<Self>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum InObjectState {
    FirstFrame,
    Inside,
    Outside,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sleep;

impl Component for Sleep {
    type Storage = NullStorage<Self>;
}

pub struct UpdateParticles<'a, H: ChunkHandlerGeneric> {
    pub chunk_handler: &'a mut H,
}

impl<'a, H: ChunkHandlerGeneric> System<'a> for UpdateParticles<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Write<'a, SimpleMarkerAllocator<FilePersistent>>,
        WriteStorage<'a, SimpleMarker<FilePersistent>>,
        WriteStorage<'a, Particle>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Hitbox>,
        ReadStorage<'a, Sleep>,
        Read<'a, LazyUpdate>,
        Read<'a, TickTime>,
    );

    #[allow(clippy::too_many_lines)]
    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateParticles::run");

        let (
            entities,
            mut marker_alloc,
            mut markers,
            mut particle,
            mut pos,
            mut vel,
            hitbox,
            sleep,
            updater,
            tick_time,
        ) = data;
        // let chunk_handler = chunk_handler.unwrap().0;
        let chunk_handler = &mut *self.chunk_handler;

        let new_p = entities.create();
        particle
            .insert(
                new_p,
                Particle {
                    material: MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Sand,
                        color: Color::RGB(64, 255, 255),
                    },
                    in_object_state: InObjectState::FirstFrame,
                },
            )
            .expect("Failed to insert Particle");
        pos.insert(
            new_p,
            Position { x: (rand::random::<f64>() - 0.5) * 10.0, y: -100.0 },
        )
        .expect("Failed to insert Position");
        vel.insert(
            new_p,
            Velocity {
                x: (rand::random::<f64>() - 0.5) * 4.0,
                y: (rand::random::<f64>() - 0.75) * 2.0,
            },
        )
        .expect("Failed to insert Velocity");
        marker_alloc.mark(new_p, &mut markers);

        if tick_time.0 % 29 == 0 {
            (&entities, &mut particle, &mut pos, !&sleep).join().for_each(|(ent, _part, pos, _)| {
                let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
                if !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                    updater.insert(ent, Sleep);
                }
            });
        } else if tick_time.0 % 29 == 10 {
            (&entities, &mut particle, &mut pos, &sleep).join().for_each(|(ent, _part, pos, _)| {
                let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
                if matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                    updater.remove::<Sleep>(ent);
                }
            });
        }

        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut particle, &mut pos, &mut vel, !&sleep)
            .join()
            .for_each(|(ent, part, pos, vel, _)| {
                // profiling::scope!("particle");

                let lx = pos.x;
                let ly = pos.y;

                vel.y += 0.1;

                let dx = vel.x;
                let dy = vel.y;

                let steps = (dx.abs() + dy.abs()).sqrt() as u32 + 1;
                {
                    // profiling::scope!("loop", format!("steps = {}", steps).as_str());

                    // let mut last_step_x = pos.x as i64;
                    // let mut last_step_y = pos.y as i64;
                    for s in 0..steps {
                        // profiling::scope!("step");
                        let thru = f64::from(s + 1) / f64::from(steps);

                        pos.x = lx + dx * thru;
                        pos.y = ly + dy * thru;

                        // this check does catch repeated steps, but actually makes performance slightly worse
                        // if pos.x as i64 != last_step_x || pos.y as i64 != last_step_y {
                        if let Ok(mat) = chunk_handler.get(pos.x as i64, pos.y as i64) {
                            if mat.physics == PhysicsType::Air {
                                part.in_object_state = InObjectState::Outside;
                            } else {
                                let is_object = mat.physics == PhysicsType::Object;

                                match part.in_object_state {
                                    InObjectState::FirstFrame => {
                                        if is_object {
                                            part.in_object_state = InObjectState::Inside;
                                        } else {
                                            part.in_object_state = InObjectState::Outside;
                                        }
                                    }
                                    InObjectState::Inside => {
                                        if !is_object {
                                            part.in_object_state = InObjectState::Outside;
                                        }
                                    }
                                    InObjectState::Outside => {}
                                }

                                if !is_object || part.in_object_state == InObjectState::Outside {
                                    match chunk_handler.get(lx as i64, ly as i64) {
                                        Ok(m) if m.physics != PhysicsType::Air => {
                                            let succeeded = chunk_handler.displace(
                                                pos.x as i64,
                                                pos.y as i64,
                                                part.material,
                                            );

                                            if succeeded {
                                                entities
                                                    .delete(ent)
                                                    .expect("Failed to delete particle");
                                            } else {
                                                // upwarp if completely blocked
                                                vel.y = -1.0;
                                                pos.y -= 16.0;
                                            }

                                            break;
                                        }
                                        _ => {
                                            if chunk_handler
                                                .set(lx as i64, ly as i64, part.material)
                                                .is_ok()
                                            {
                                                entities
                                                    .delete(ent)
                                                    .expect("Failed to delete particle");
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // }

                        // last_step_x = pos.x as i64;
                        // last_step_y = pos.y as i64;
                    }
                }
            });

        (&entities, &mut particle, &pos)
            .join()
            .for_each(|(ent, _part, my_pos)| {
                // profiling::scope!("Particle");

                // let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(my_pos.x as i64, my_pos.y as i64);
                // // skip if chunk not active
                // if !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                //     return;
                // }

                (&entities, &hitbox, &pos, !&sleep)
                    .join()
                    .for_each(|(p_ent, hb, pos, _)| {
                        if my_pos.x >= f64::from(hb.x1) + pos.x
                            && my_pos.y >= f64::from(hb.y1) + pos.y
                            && my_pos.x < f64::from(hb.x2) + pos.x
                            && my_pos.y < f64::from(hb.y2) + pos.y
                        {
                            let p = vel.get(p_ent).cloned();
                            let mp = vel.get_mut(ent);
                            if let (Some(mp), Some(p)) = (mp, p) {
                                mp.x += (-p.x - mp.x) * 0.5
                                    + rand::distributions::Uniform::from(-1.0..=1.0)
                                        .sample(&mut rand::thread_rng());
                                mp.y += (-p.y - mp.y) * 0.25
                                    + rand::distributions::Uniform::from(-1.0..=1.0)
                                        .sample(&mut rand::thread_rng());
                            }
                        }
                    });
            });
    }
}
