use std::{collections::HashMap, hash::BuildHasherDefault, sync::Arc};

use super::{
    entity::Hitbox, material::MaterialInstance, ChunkHandlerGeneric, Position, TickTime, Velocity,
};
use crate::game::common::world::{
    chunk_index, chunk_update_order,
    material::{self, color::Color, PhysicsType},
    pixel_to_chunk_pos, pixel_to_chunk_pos_with_chunk_size, ChunkState, PassThroughHasherU32,
};

use itertools::Itertools;
use rand::prelude::Distribution;
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use serde::{Deserialize, Serialize};
use specs::{Entities, Join, Read, ReadStorage, System, Write};

const PARTICLE_CHUNK_SIZE: u16 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    pub material: MaterialInstance,
    pub pos: Position,
    pub vel: Velocity,
    pub in_object_state: InObjectState,
    pub chunk_cache: (u32, u8), // (chunk index, chunk update order) TODO: make chunk update order a type?
}

impl Particle {
    pub fn new(material: MaterialInstance, pos: Position, vel: Velocity) -> Self {
        let (chunk_x, chunk_y) =
            pixel_to_chunk_pos_with_chunk_size(pos.x as i64, pos.y as i64, PARTICLE_CHUNK_SIZE);
        Self {
            material,
            chunk_cache: (
                chunk_index(chunk_x, chunk_y),
                chunk_update_order(chunk_x, chunk_y),
            ),
            pos,
            vel,
            in_object_state: InObjectState::FirstFrame,
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct Particle {
//     pub material: MaterialInstance,
//     in_object_state: InObjectState,
// }

// impl Particle {
//     pub fn of(material: MaterialInstance) -> Self {
//         Self {
//             material,
//             in_object_state: InObjectState::FirstFrame,
//         }
//     }
// }

// impl Component for Particle {
//     type Storage = VecStorage<Self>;
// }

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum InObjectState {
    FirstFrame,
    Inside,
    Outside,
}

// #[derive(Debug, Default, Clone, Serialize, Deserialize)]
// pub struct Sleep;

// impl Component for Sleep {
//     type Storage = NullStorage<Self>;
// }

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ParticleSystem {
    pub active: Vec<Particle>,
    pub sleeping: Vec<Particle>,
}

pub struct UpdateParticles<'a, H: ChunkHandlerGeneric + Send + Sync> {
    pub chunk_handler: &'a mut H,
}

impl<'a, H: ChunkHandlerGeneric + Send + Sync> System<'a> for UpdateParticles<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Write<'a, ParticleSystem>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, Hitbox>,
        Read<'a, TickTime>,
    );

    #[allow(clippy::too_many_lines)]
    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut system, pos, vel, hitbox, tick_time) = data;
        profiling::scope!(
            "UpdateParticles::run",
            format!("n = {}/{}", system.active.len(), system.sleeping.len()).as_str()
        );

        let chunk_handler = &mut *self.chunk_handler;

        system.active.push(Particle::new(
            MaterialInstance {
                material_id: material::TEST,
                physics: PhysicsType::Sand,
                color: Color::rgb(64, 255, 255),
            },
            Position { x: (rand::random::<f64>() - 0.5) * 10.0, y: -100.0 },
            Velocity {
                x: (rand::random::<f64>() - 0.5) * 4.0,
                y: (rand::random::<f64>() - 0.75) * 2.0,
            },
        ));

        if tick_time.0 % 29 == 0 {
            profiling::scope!("active->sleep");
            // TODO: we want to use the std version once it is stable
            use drain_filter_polyfill::VecExt;
            #[allow(unstable_name_collisions)]
            let mut removed = system.active.drain_filter(|p| {
                let (chunk_x, chunk_y) = pixel_to_chunk_pos(p.pos.x as i64, p.pos.y as i64);
                !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active)
            }).collect::<Vec<_>>();
            system.sleeping.append(&mut removed);
        } else if tick_time.0 % 29 == 10 {
            profiling::scope!("sleep->active");
            // TODO: we want to use the std version once it is stable
            use drain_filter_polyfill::VecExt;
            #[allow(unstable_name_collisions)]
            let mut removed = system.sleeping.drain_filter(|p| {
                let (chunk_x, chunk_y) = pixel_to_chunk_pos(p.pos.x as i64, p.pos.y as i64);
                matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active)
            }).collect::<Vec<_>>();
            system.active.append(&mut removed);
        }

        {
            profiling::scope!("main");

            // ok so there's some pretty unsafe stuff going on here in order to get parallelism working
            // chunk_handler is mutably borrowed many times on different threads in order to call get, set, and displace
            // I don't know if HashMap::get is "thread-sound". Assuming it is, the reads/writes to the chunk buffer arrays
            //   should be sound assuming our invariant of no particle being allowed to move > CHUNK_SIZE/2 per tick (TODO: this is not enforced anywhere)
            // TODO: look into replacing the chunks HashMap with https://crates.io/crates/evmap or https://crates.io/crates/chashmap so it's actually sound

            struct ForceSendSync<T> {
                value: T,
            }

            unsafe impl<T> Send for ForceSendSync<T> {}
            unsafe impl<T> Sync for ForceSendSync<T> {}

            let async_chunk_handler =
                Arc::new(ForceSendSync::<*mut &mut H> { value: &mut self.chunk_handler });

            let parts: Vec<_> = {
                profiling::scope!("sort+group");
                let mut maps = [
                    HashMap::<u32, Vec<Particle>, BuildHasherDefault<PassThroughHasherU32>>::default(),
                    HashMap::<u32, Vec<Particle>, BuildHasherDefault<PassThroughHasherU32>>::default(),
                    HashMap::<u32, Vec<Particle>, BuildHasherDefault<PassThroughHasherU32>>::default(),
                    HashMap::<u32, Vec<Particle>, BuildHasherDefault<PassThroughHasherU32>>::default(),
                ];
                for p in system.active.drain(..) {
                    // safety: p.chunk_cache.1 is a chunk order number, assumed to be 0..=3
                    unsafe { maps.get_unchecked_mut(p.chunk_cache.1 as usize) }
                        .entry(p.chunk_cache.0)
                        .or_insert_with(Vec::new)
                        .push(p);
                }
                maps.into_iter()
                    .map(|m| m.into_values().collect_vec())
                    .collect()
            };

            for p in parts {
                profiling::scope!("phase", format!("n_chunks = {}", p.len()).as_str());
                let v = p.into_par_iter().flat_map_iter(|mut chunk_px| {
                    profiling::scope!("chunk");

                    use retain_mut::RetainMut;
                    #[allow(unstable_name_collisions)]
                    chunk_px.retain_mut(|part| {
                        let unsafe_async_chunk_handler =
                            unsafe { &mut **((async_chunk_handler.clone()).value) };
                        let chunk_handler = unsafe_async_chunk_handler;

                        let mut process = |part: &mut Particle| {
                            let lx = part.pos.x;
                            let ly = part.pos.y;

                            part.vel.y += 0.1;

                            let dx = part.vel.x;
                            let dy = part.vel.y;

                            let steps = (dx.abs() + dy.abs()).sqrt() as u32 + 1;
                            {
                                // profiling::scope!("loop", format!("steps = {}", steps).as_str());

                                // let mut last_step_x = pos.x as i64;
                                // let mut last_step_y = pos.y as i64;
                                for s in 0..steps {
                                    // profiling::scope!("step");
                                    let thru = f64::from(s + 1) / f64::from(steps);

                                    part.pos.x = lx + dx * thru;
                                    part.pos.y = ly + dy * thru;

                                    // this check does catch repeated steps, but actually makes performance slightly worse
                                    // if pos.x as i64 != last_step_x || pos.y as i64 != last_step_y {
                                    if let Ok(mat) =
                                        chunk_handler.get(part.pos.x as i64, part.pos.y as i64)
                                    {
                                        if mat.physics == PhysicsType::Air {
                                            part.in_object_state = InObjectState::Outside;
                                        } else {
                                            let is_object = mat.physics == PhysicsType::Object;

                                            match part.in_object_state {
                                                InObjectState::FirstFrame => {
                                                    if is_object {
                                                        part.in_object_state =
                                                            InObjectState::Inside;
                                                    } else {
                                                        part.in_object_state =
                                                            InObjectState::Outside;
                                                    }
                                                },
                                                InObjectState::Inside => {
                                                    if !is_object {
                                                        part.in_object_state =
                                                            InObjectState::Outside;
                                                    }
                                                },
                                                InObjectState::Outside => {},
                                            }

                                            if !is_object
                                                || part.in_object_state == InObjectState::Outside
                                            {
                                                match chunk_handler.get(lx as i64, ly as i64) {
                                                    Ok(m) if m.physics != PhysicsType::Air => {
                                                        let succeeded = chunk_handler.displace(
                                                            part.pos.x as i64,
                                                            part.pos.y as i64,
                                                            part.material,
                                                        );

                                                        if succeeded {
                                                            return false;
                                                        }

                                                        // upwarp if completely blocked
                                                        part.vel.y = -1.0;
                                                        part.pos.y -= 16.0;

                                                        break;
                                                    },
                                                    _ => {
                                                        if chunk_handler
                                                            .set(
                                                                lx as i64,
                                                                ly as i64,
                                                                part.material,
                                                            )
                                                            .is_ok()
                                                        {
                                                            return false;
                                                        }
                                                    },
                                                }
                                            }
                                        }
                                    }
                                    // }

                                    // last_step_x = pos.x as i64;
                                    // last_step_y = pos.y as i64;
                                }
                            }

                            true
                        };

                        let res = process(part);
                        let (chunk_x, chunk_y) = pixel_to_chunk_pos_with_chunk_size(
                            part.pos.x as i64,
                            part.pos.y as i64,
                            PARTICLE_CHUNK_SIZE,
                        );
                        part.chunk_cache = (
                            chunk_index(chunk_x, chunk_y),
                            chunk_update_order(chunk_x, chunk_y),
                        );
                        res
                    });

                    chunk_px
                });

                {
                    profiling::scope!("par_extend");
                    system.active.par_extend(v);
                }

                // let parts = system.active.iter_mut().filter(|p| {
                //     let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(p.pos.x as i64, p.pos.y as i64);
                //     chunk_handler.chunk_update_order(chunk_x, chunk_y) == phase
                // });

                // for p in parts {

                // }
            }
        }

        // TODO: we want to use the std version once it is stable
        //     use retain_mut::RetainMut;
        //     #[allow(unstable_name_collisions)]
        //     system.active.retain_mut(|part| {
        //         // profiling::scope!("particle");

        //         let lx = part.pos.x;
        //         let ly = part.pos.y;

        //         part.vel.y += 0.1;

        //         let dx = part.vel.x;
        //         let dy = part.vel.y;

        //         let steps = (dx.abs() + dy.abs()).sqrt() as u32 + 1;
        //         {
        //             // profiling::scope!("loop", format!("steps = {}", steps).as_str());

        //             // let mut last_step_x = pos.x as i64;
        //             // let mut last_step_y = pos.y as i64;
        //             for s in 0..steps {
        //                 // profiling::scope!("step");
        //                 let thru = f64::from(s + 1) / f64::from(steps);

        //                 part.pos.x = lx + dx * thru;
        //                 part.pos.y = ly + dy * thru;

        //                 // this check does catch repeated steps, but actually makes performance slightly worse
        //                 // if pos.x as i64 != last_step_x || pos.y as i64 != last_step_y {
        //                 if let Ok(mat) = chunk_handler.get(part.pos.x as i64, part.pos.y as i64) {
        //                     if mat.physics == PhysicsType::Air {
        //                         part.in_object_state = InObjectState::Outside;
        //                     } else {
        //                         let is_object = mat.physics == PhysicsType::Object;

        //                         match part.in_object_state {
        //                             InObjectState::FirstFrame => {
        //                                 if is_object {
        //                                     part.in_object_state = InObjectState::Inside;
        //                                 } else {
        //                                     part.in_object_state = InObjectState::Outside;
        //                                 }
        //                             },
        //                             InObjectState::Inside => {
        //                                 if !is_object {
        //                                     part.in_object_state = InObjectState::Outside;
        //                                 }
        //                             },
        //                             InObjectState::Outside => {},
        //                         }

        //                         if !is_object || part.in_object_state == InObjectState::Outside {
        //                             match chunk_handler.get(lx as i64, ly as i64) {
        //                                 Ok(m) if m.physics != PhysicsType::Air => {
        //                                     let succeeded = chunk_handler.displace(
        //                                         part.pos.x as i64,
        //                                         part.pos.y as i64,
        //                                         part.material,
        //                                     );

        //                                     if succeeded {
        //                                         return false;
        //                                     }

        //                                     // upwarp if completely blocked
        //                                     part.vel.y = -1.0;
        //                                     part.pos.y -= 16.0;

        //                                     break;
        //                                 },
        //                                 _ => {
        //                                     if chunk_handler
        //                                         .set(lx as i64, ly as i64, part.material)
        //                                         .is_ok()
        //                                     {
        //                                         return false;
        //                                     }
        //                                 },
        //                             }
        //                         }
        //                     }
        //                 }
        //                 // }

        //                 // last_step_x = pos.x as i64;
        //                 // last_step_y = pos.y as i64;
        //             }
        //         }

        //         true
        //     });
        // }

        {
            profiling::scope!("ent");
            for part in &mut system.active {
                // profiling::scope!("Particle");

                // let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(my_pos.x as i64, my_pos.y as i64);
                // // skip if chunk not active
                // if !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                //     return;
                // }

                (&entities, &hitbox, &pos)
                    .join()
                    .for_each(|(p_ent, hb, pos)| {
                        if part.pos.x >= f64::from(hb.x1) + pos.x
                            && part.pos.y >= f64::from(hb.y1) + pos.y
                            && part.pos.x < f64::from(hb.x2) + pos.x
                            && part.pos.y < f64::from(hb.y2) + pos.y
                        {
                            let p = vel.get(p_ent).cloned();
                            let mp = Some(&mut part.vel);
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
            }
        }
    }
}
