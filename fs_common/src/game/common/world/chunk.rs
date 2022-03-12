use crate::game::common::world::particle::ParticleSystem;
use crate::game::common::world::simulator::Simulator;
use crate::game::common::world::{Loader, Position};
use crate::game::common::{Settings, Rect};
use std::collections::HashMap;
use std::convert::TryInto;
use std::hash::BuildHasherDefault;
use std::path::PathBuf;

use rapier2d::prelude::{Collider, RigidBody, RigidBodyHandle};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use specs::{Join, ReadStorage, WorldExt};

use super::gen::WorldGenerator;
use super::material::{PhysicsType, Color};
use super::particle::Particle;
use super::physics::Physics;
use crate::game::common::world::material::MaterialInstance;

pub const CHUNK_SIZE: u16 = 100;

#[warn(clippy::large_enum_variant)]
pub enum RigidBodyState {
    Active(RigidBodyHandle),
    Inactive(Box<RigidBody>, Vec<Collider>),
}

pub trait Chunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self
    where
        Self: Sized;

    fn get_chunk_x(&self) -> i32;
    fn get_chunk_y(&self) -> i32;

    fn get_state(&self) -> ChunkState;
    fn set_state(&mut self, state: ChunkState);

    fn get_dirty_rect(&self) -> Option<Rect>;
    fn set_dirty_rect(&mut self, rect: Option<Rect>);

    fn set_pixels(&mut self, pixels: &[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]);
    fn get_pixels_mut(
        &mut self,
    ) -> &mut Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>;
    fn get_pixels(&self) -> &Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>;
    fn set_pixel_colors(&mut self, colors: &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]);
    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];
    fn get_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];

    fn generate_mesh(&mut self) -> Result<(), String>;
    // fn get_tris(&self) -> &Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>>;
    fn get_mesh_loops(&self) -> &Option<Vec<Vec<Vec<Vec<f64>>>>>;
    fn get_rigidbody(&self) -> &Option<RigidBodyState>;
    fn get_rigidbody_mut(&mut self) -> &mut Option<RigidBodyState>;
    fn set_rigidbody(&mut self, body: Option<RigidBodyState>);

    fn mark_dirty(&mut self);

    fn refresh(&mut self);
    fn update_graphics(&mut self) -> Result<(), String>;
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String>;
    fn get(&self, x: u16, y: u16) -> Result<&MaterialInstance, String>;
    fn set_color(&mut self, x: u16, y: u16, color: Color) -> Result<(), String>;
    fn get_color(&self, x: u16, y: u16) -> Result<Color, String>;
    fn apply_diff(&mut self, diff: &[(u16, u16, MaterialInstance)]);
}

#[derive(Clone, Copy, PartialEq)]
pub enum ChunkState {
    NotGenerated,
    Generating(u8), // stage
    Cached,
    Active,
}

#[derive(Debug)]
pub struct ChunkHandler<T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> {
    pub loaded_chunks: HashMap<u32, C, BuildHasherDefault<PassThroughHasherU32>>,
    pub load_queue: Vec<(i32, i32)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: T,
    pub path: Option<PathBuf>,
}

unsafe impl<T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> Send
    for ChunkHandler<T, C>
{
}
unsafe impl<T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> Sync
    for ChunkHandler<T, C>
{
}

pub trait ChunkHandlerGeneric {
    fn update_chunk_graphics(&mut self);
    fn tick(
        &mut self,
        tick_time: u32,
        settings: &Settings,
        world: &mut specs::World,
        physics: &mut Physics,
    );
    fn save_chunk(&mut self, index: u32) -> Result<(), Box<dyn std::error::Error>>;
    fn unload_all_chunks(
        &mut self,
        physics: &mut Physics,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn save_all_chunks(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn queue_load_chunk(&mut self, chunk_x: i32, chunk_y: i32) -> bool;
    fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool;
    fn is_pixel_loaded(&self, x: i64, y: i64) -> bool;
    fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&dyn Chunk>;
    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_y: i32) -> Option<&mut dyn Chunk>;
    fn set(&mut self, x: i64, y: i64, mat: MaterialInstance) -> Result<(), String>;
    fn get(&self, x: i64, y: i64) -> Result<&MaterialInstance, String>;
    fn displace(&mut self, x: i64, y: i64, material: MaterialInstance) -> bool;
    fn force_update_chunk(&mut self, chunk_x: i32, chunk_y: i32);
    fn get_zone(&self, center: (f64, f64), padding: u16) -> Rect;
    fn get_screen_zone(&self, center: (f64, f64)) -> Rect;
    fn get_active_zone(&self, center: (f64, f64)) -> Rect;
    fn get_load_zone(&self, center: (f64, f64)) -> Rect;
    fn get_unload_zone(&self, center: (f64, f64)) -> Rect;
}

#[derive(Serialize, Deserialize)]
struct ChunkSaveFormat {
    pixels: Vec<MaterialInstance>,
    colors: Vec<u8>,
}

impl<'a, T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> ChunkHandlerGeneric
    for ChunkHandler<T, C>
{
    #[profiling::function]
    fn update_chunk_graphics(&mut self) {
        let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
        for key in keys {
            self.loaded_chunks
                .get_mut(&key)
                .unwrap()
                .update_graphics()
                .unwrap();
        }
    }

    // #[profiling::function] // breaks clippy
    #[allow(clippy::too_many_lines)]
    fn tick(
        &mut self,
        tick_time: u32,
        settings: &Settings,
        world: &mut specs::World,
        physics: &mut Physics,
    ) {
        profiling::scope!("tick");

        let (loaders, positions) =
            world.system_data::<(ReadStorage<Loader>, ReadStorage<Position>)>();

        let unload_zone: Vec<Rect> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_unload_zone((p.x, p.y)))
            .collect();
        let load_zone: Vec<Rect> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_load_zone((p.x, p.y)))
            .collect();
        let active_zone: Vec<Rect> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_active_zone((p.x, p.y)))
            .collect();
        let _screen_zone: Vec<Rect> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_screen_zone((p.x, p.y)))
            .collect();

        if settings.load_chunks {
            {
                profiling::scope!("queue chunk loading");
                for load_zone in load_zone {
                    for px in load_zone.range_lr().step_by(CHUNK_SIZE.into()) {
                        for py in load_zone.range_tb().step_by(CHUNK_SIZE.into())
                        {
                            let chunk_pos = pixel_to_chunk_pos(px.into(), py.into());
                            self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
                        }
                    }
                }
            }

            {
                profiling::scope!("chunk loading");
                for _ in 0..64 {
                    // TODO: don't load queued chunks if they are no longer in range
                    if let Some(to_load) = self.load_queue.pop() {
                        self.load_chunk(to_load.0, to_load.1);
                    }
                }
            }
        }

        // switch chunks between cached and active
        if tick_time % 2 == 0 {
            profiling::scope!("chunk update A");

            let mut keep_map = vec![true; self.loaded_chunks.len()];
            let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
            for i in 0..keys.len() {
                let key = keys[i];

                let state = self.loaded_chunks.get(&key).unwrap().get_state(); // copy
                let rect = Rect::new(
                    self.loaded_chunks.get(&key).unwrap().get_chunk_x() * i32::from(CHUNK_SIZE),
                    self.loaded_chunks.get(&key).unwrap().get_chunk_y() * i32::from(CHUNK_SIZE),
                    u32::from(CHUNK_SIZE),
                    u32::from(CHUNK_SIZE),
                );

                match state {
                    ChunkState::Cached => {
                        if !unload_zone.iter().any(|z| rect.intersects(z)) {
                            if let Err(e) = self.save_chunk(key) {
                                log::error!(
                                    "Chunk @ {}, {} failed to save: {:?}",
                                    chunk_index_inv(key).0,
                                    chunk_index_inv(key).1,
                                    e
                                );
                            };
                            if let Err(e) = self.unload_chunk(key, physics) {
                                log::error!(
                                    "Chunk @ {}, {} failed to unload: {:?}",
                                    chunk_index_inv(key).0,
                                    chunk_index_inv(key).1,
                                    e
                                );
                            };
                            keep_map[i] = false;
                        } else if active_zone.iter().any(|z| rect.intersects(z)) {
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().get_chunk_x();
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().get_chunk_y();
                            if [
                                self.get_chunk(chunk_x - 1, chunk_y - 1),
                                self.get_chunk(chunk_x, chunk_y - 1),
                                self.get_chunk(chunk_x + 1, chunk_y - 1),
                                self.get_chunk(chunk_x - 1, chunk_y),
                                self.get_chunk(chunk_x, chunk_y),
                                self.get_chunk(chunk_x + 1, chunk_y),
                                self.get_chunk(chunk_x - 1, chunk_y + 1),
                                self.get_chunk(chunk_x, chunk_y + 1),
                                self.get_chunk(chunk_x + 1, chunk_y + 1),
                            ]
                            .iter()
                            .all(|ch| {
                                if ch.is_none() {
                                    return false;
                                }

                                let state = ch.unwrap().get_state();

                                matches!(state, ChunkState::Cached | ChunkState::Active)
                            }) {
                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_state(ChunkState::Active);
                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_dirty_rect(Some(Rect::new(
                                        0,
                                        0,
                                        u32::from(CHUNK_SIZE),
                                        u32::from(CHUNK_SIZE),
                                    )));
                            }
                        }
                    },
                    ChunkState::Active => {
                        if !active_zone.iter().any(|z| rect.intersects(z)) {
                            self.loaded_chunks
                                .get_mut(&key)
                                .unwrap()
                                .set_state(ChunkState::Cached);
                        }
                    },
                    _ => {},
                }
            }

            if settings.load_chunks {
                let mut iter = keep_map.iter();
                self.loaded_chunks.retain(|_, _| *iter.next().unwrap());
            }
        }

        if settings.load_chunks {
            // generate new chunks
            if tick_time % 2 == 0 {
                profiling::scope!("chunk update B");

                let mut num_loaded_this_tick = 0;

                let mut keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
                if !loaders.is_empty() {
                    keys.sort_by(|a, b| {
                        let c1_x = self.loaded_chunks.get(a).unwrap().get_chunk_x()
                            * i32::from(CHUNK_SIZE);
                        let c1_y = self.loaded_chunks.get(a).unwrap().get_chunk_y()
                            * i32::from(CHUNK_SIZE);
                        let c2_x = self.loaded_chunks.get(b).unwrap().get_chunk_x()
                            * i32::from(CHUNK_SIZE);
                        let c2_y = self.loaded_chunks.get(b).unwrap().get_chunk_y()
                            * i32::from(CHUNK_SIZE);

                        let d1 = (&loaders, &positions)
                            .join()
                            .map(|(_, p)| {
                                let x = (p.x as i32 - c1_x).abs();
                                let y = (p.y as i32 - c1_y).abs();
                                x + y
                            })
                            .min()
                            .unwrap();

                        let d2 = (&loaders, &positions)
                            .join()
                            .map(|(_, p)| {
                                let x = (p.x as i32 - c2_x).abs();
                                let y = (p.y as i32 - c2_y).abs();
                                x + y
                            })
                            .min()
                            .unwrap();

                        d1.cmp(&d2)
                    });
                }
                let mut to_exec = vec![];
                for (i, key) in keys.iter().enumerate() {
                    let state = self.loaded_chunks.get(key).unwrap().get_state(); // copy
                    let rect = Rect::new(
                        self.loaded_chunks.get(key).unwrap().get_chunk_x() * i32::from(CHUNK_SIZE),
                        self.loaded_chunks.get(key).unwrap().get_chunk_y() * i32::from(CHUNK_SIZE),
                        u32::from(CHUNK_SIZE),
                        u32::from(CHUNK_SIZE),
                    );

                    if state == ChunkState::NotGenerated {
                        if !unload_zone.iter().any(|z| rect.intersects(z)) {
                        } else if num_loaded_this_tick < 32 {
                            let chunk_x = self.loaded_chunks.get_mut(key).unwrap().get_chunk_x();
                            let chunk_y = self.loaded_chunks.get_mut(key).unwrap().get_chunk_y();

                            let mut should_generate = true;

                            if let Some(path) = &self.path {
                                let chunk_path_root = path.join("chunks/");
                                if !chunk_path_root.exists() {
                                    std::fs::create_dir_all(&chunk_path_root).expect(
                                        format!(
                                            "Failed to create chunk directory @ {:?}",
                                            chunk_path_root
                                        )
                                        .as_str(),
                                    );
                                }
                                let chunk_path =
                                    chunk_path_root.join(format!("{}_{}.chunk", chunk_x, chunk_y));
                                if chunk_path.exists() {
                                    if let Ok(data) = std::fs::read(&chunk_path) {
                                        match bincode::deserialize(&data) {
                                            Ok(res) => {
                                                let save: ChunkSaveFormat = res;

                                                if save.pixels.len()
                                                    == (CHUNK_SIZE as usize * CHUNK_SIZE as usize)
                                                        as usize
                                                {
                                                    self.loaded_chunks
                                                        .get_mut(key)
                                                        .unwrap()
                                                        .set_state(ChunkState::Cached);
                                                    self.loaded_chunks
                                                        .get_mut(key)
                                                        .unwrap()
                                                        .set_pixels(
                                                            &save.pixels.try_into().unwrap(),
                                                        );
                                                    self.loaded_chunks
                                                        .get_mut(key)
                                                        .unwrap()
                                                        .mark_dirty();
                                                    let _ = self
                                                        .loaded_chunks
                                                        .get_mut(key)
                                                        .unwrap()
                                                        .generate_mesh();

                                                    if save.colors.len()
                                                        == (CHUNK_SIZE as usize
                                                            * CHUNK_SIZE as usize
                                                            * 4)
                                                            as usize
                                                    {
                                                        self.loaded_chunks
                                                            .get_mut(key)
                                                            .unwrap()
                                                            .set_pixel_colors(
                                                                &save.colors.try_into().unwrap(),
                                                            );
                                                    } else {
                                                        log::error!("colors Vec is the wrong size: {} (expected {})", save.colors.len(), CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4);
                                                        self.loaded_chunks
                                                            .get_mut(key)
                                                            .unwrap()
                                                            .refresh();
                                                    }

                                                    should_generate = false;
                                                } else {
                                                    log::error!("pixels Vec is the wrong size: {} (expected {})", save.pixels.len(), CHUNK_SIZE * CHUNK_SIZE);
                                                    self.loaded_chunks
                                                        .get_mut(key)
                                                        .unwrap()
                                                        .set_state(ChunkState::Cached);
                                                }
                                            },
                                            Err(e) => {
                                                log::error!(
                                                    "Chunk parse failed @ {},{} -> {:?}: {:?}",
                                                    chunk_x,
                                                    chunk_y,
                                                    chunk_path,
                                                    e
                                                );
                                                self.loaded_chunks
                                                    .get_mut(key)
                                                    .unwrap()
                                                    .set_state(ChunkState::Cached);
                                            },
                                        }
                                    } else {
                                        log::error!(
                                            "Chunk load failed @ {},{} -> {:?}",
                                            chunk_x,
                                            chunk_y,
                                            chunk_path
                                        );
                                        self.loaded_chunks
                                            .get_mut(key)
                                            .unwrap()
                                            .set_state(ChunkState::Cached);
                                    }
                                }
                            }

                            if should_generate {
                                self.loaded_chunks
                                    .get_mut(key)
                                    .unwrap()
                                    .set_state(ChunkState::Generating(0));
                                to_exec.push((i, chunk_x, chunk_y));
                            }

                            // generation_pool.spawn_ok(fut);
                            num_loaded_this_tick += 1;
                        }
                    }
                }

                if !to_exec.is_empty() {
                    profiling::scope!("gen chunks");
                    // println!("a {}", to_exec.len());

                    let b = {
                        profiling::scope!("gen");
                        let gen = self.generator;
                        let futs2: Vec<_> = to_exec
                            .into_par_iter()
                            .map(|e| {
                                profiling::register_thread!("Generation thread");
                                profiling::scope!("chunk");

                                // these arrays are too large for the stack

                                let mut pixels = Box::new(
                                    [MaterialInstance::air(); (CHUNK_SIZE * CHUNK_SIZE) as usize],
                                );

                                #[allow(clippy::cast_lossless)]
                                let mut colors = Box::new(
                                    [0; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize],
                                );

                                gen.generate(e.1, e.2, 2, &mut pixels, &mut colors); // TODO: non constant seed
                                                                                     // println!("{}", e.0);
                                (e.0, pixels, colors)
                            })
                            .collect();
                        futs2
                    };
                    for r in b {
                        profiling::scope!("finish chunk");

                        // TODO: this is very slow

                        let p = r;
                        // println!("{} {}", i, p.0);
                        let chunk = self.loaded_chunks.get_mut(&keys[p.0]).unwrap();
                        chunk.set_pixels(&p.1);
                        chunk.set_pixel_colors(&p.2);
                        let _ = chunk.generate_mesh();
                    }
                }
            }

            // unloading NotGenerated or Generating chunks
            // populate chunks
            if tick_time % 2 == 0 {
                profiling::scope!("chunk update C");

                let mut keep_map = vec![true; self.loaded_chunks.len()];
                let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().get_state(); // copy
                    let rect = Rect::new(
                        self.loaded_chunks.get(&key).unwrap().get_chunk_x() * i32::from(CHUNK_SIZE),
                        self.loaded_chunks.get(&key).unwrap().get_chunk_y() * i32::from(CHUNK_SIZE),
                        u32::from(CHUNK_SIZE),
                        u32::from(CHUNK_SIZE),
                    );

                    match state {
                        ChunkState::NotGenerated => {
                            if !unload_zone.iter().any(|z| rect.intersects(z)) {
                                if let Err(e) = self.save_chunk(key) {
                                    log::error!(
                                        "Chunk @ {}, {} failed to save: {:?}",
                                        chunk_index_inv(key).0,
                                        chunk_index_inv(key).1,
                                        e
                                    );
                                };
                                if let Err(e) = self.unload_chunk(key, physics) {
                                    log::error!(
                                        "Chunk @ {}, {} failed to unload: {:?}",
                                        chunk_index_inv(key).0,
                                        chunk_index_inv(key).1,
                                        e
                                    );
                                };
                                keep_map[i] = false;
                            }
                        },
                        ChunkState::Generating(cur_stage) => {
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().get_chunk_x();
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().get_chunk_y();

                            let max_stage = self.generator.max_gen_stage();

                            if cur_stage >= max_stage {
                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_state(ChunkState::Cached);
                            } else {
                                if [
                                    self.get_chunk(chunk_x - 1, chunk_y - 1),
                                    self.get_chunk(chunk_x, chunk_y - 1),
                                    self.get_chunk(chunk_x + 1, chunk_y - 1),
                                    self.get_chunk(chunk_x - 1, chunk_y),
                                    self.get_chunk(chunk_x, chunk_y),
                                    self.get_chunk(chunk_x + 1, chunk_y),
                                    self.get_chunk(chunk_x - 1, chunk_y + 1),
                                    self.get_chunk(chunk_x, chunk_y + 1),
                                    self.get_chunk(chunk_x + 1, chunk_y + 1),
                                ]
                                .iter()
                                .all(|ch| {
                                    if ch.is_none() {
                                        return false;
                                    }

                                    let state = ch.unwrap().get_state();

                                    match state {
                                        ChunkState::Cached | ChunkState::Active => true,
                                        ChunkState::Generating(st) if st >= cur_stage => true,
                                        _ => false,
                                    }
                                }) {
                                    self.loaded_chunks
                                        .get_mut(&key)
                                        .unwrap()
                                        .set_state(ChunkState::Generating(cur_stage + 1));
                                }

                                if !unload_zone.iter().any(|z| rect.intersects(z)) {
                                    if let Err(e) = self.save_chunk(key) {
                                        log::error!(
                                            "Chunk @ {}, {} failed to save: {:?}",
                                            chunk_index_inv(key).0,
                                            chunk_index_inv(key).1,
                                            e
                                        );
                                    };
                                    if let Err(e) = self.unload_chunk(key, physics) {
                                        log::error!(
                                            "Chunk @ {}, {} failed to unload: {:?}",
                                            chunk_index_inv(key).0,
                                            chunk_index_inv(key).1,
                                            e
                                        );
                                    };
                                    keep_map[i] = false;
                                }
                            }
                        },
                        _ => {},
                    }
                }

                let mut iter = keep_map.iter();
                self.loaded_chunks.retain(|_, _| *iter.next().unwrap());
            }
        }

        drop(loaders);
        drop(positions);

        if settings.simulate_chunks {
            profiling::scope!("chunk simulate");

            let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
            let mut old_dirty_rects: HashMap<u32, Option<Rect>> =
                HashMap::with_capacity(keys.len());

            for key in &keys {
                old_dirty_rects.insert(*key, self.loaded_chunks.get(key).unwrap().get_dirty_rect());
                self.loaded_chunks
                    .get_mut(key)
                    .unwrap()
                    .set_dirty_rect(None);
            }

            for tick_phase in 0..4 {
                profiling::scope!("phase", format!("phase {}", tick_phase).as_str());
                let mut to_exec = vec![];
                for key in &keys {
                    let state = self.loaded_chunks.get(key).unwrap().get_state(); // copy
                    let ch_pos = (
                        self.loaded_chunks.get(key).unwrap().get_chunk_x(),
                        self.loaded_chunks.get(key).unwrap().get_chunk_y(),
                    );

                    if chunk_update_order(ch_pos.0, ch_pos.1) == tick_phase
                        && state == ChunkState::Active
                    {
                        profiling::scope!("iter");

                        if old_dirty_rects.get(key).is_some() {
                            let ch00: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1 - 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch10: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1 - 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch20: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1 - 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch01: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch11: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch21: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch02: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1 + 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch12: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1 + 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let ch22: *mut [MaterialInstance;
                                (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1 + 1))
                                .unwrap()
                                .get_pixels_mut()
                                .as_mut()
                                .unwrap();
                            let arr = [
                                ch00 as usize,
                                ch10 as usize,
                                ch20 as usize,
                                ch01 as usize,
                                ch11 as usize,
                                ch21 as usize,
                                ch02 as usize,
                                ch12 as usize,
                                ch22 as usize,
                            ];

                            let gr_ch00: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1 - 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch10: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1 - 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch20: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1 - 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch01: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch11: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch21: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch02: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 - 1, ch_pos.1 + 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch12: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0, ch_pos.1 + 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_ch22: *mut [u8; (CHUNK_SIZE as usize
                                * CHUNK_SIZE as usize
                                * 4)] = self
                                .loaded_chunks
                                .get_mut(&chunk_index(ch_pos.0 + 1, ch_pos.1 + 1))
                                .unwrap()
                                .get_colors_mut();
                            let gr_arr = [
                                gr_ch00 as usize,
                                gr_ch10 as usize,
                                gr_ch20 as usize,
                                gr_ch01 as usize,
                                gr_ch11 as usize,
                                gr_ch21 as usize,
                                gr_ch02 as usize,
                                gr_ch12 as usize,
                                gr_ch22 as usize,
                            ];

                            let dirty_ch00 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 - 1, ch_pos.1 - 1))
                                .unwrap();
                            let dirty_ch10 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0, ch_pos.1 - 1))
                                .unwrap();
                            let dirty_ch20 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 + 1, ch_pos.1 - 1))
                                .unwrap();
                            let dirty_ch01 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 - 1, ch_pos.1))
                                .unwrap();
                            let dirty_ch11 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0, ch_pos.1))
                                .unwrap();
                            let dirty_ch21 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 + 1, ch_pos.1))
                                .unwrap();
                            let dirty_ch02 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 - 1, ch_pos.1 + 1))
                                .unwrap();
                            let dirty_ch12 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0, ch_pos.1 + 1))
                                .unwrap();
                            let dirty_ch22 = *old_dirty_rects
                                .get(&chunk_index(ch_pos.0 + 1, ch_pos.1 + 1))
                                .unwrap();
                            let dirty_arr = [
                                dirty_ch00, dirty_ch10, dirty_ch20, dirty_ch01, dirty_ch11,
                                dirty_ch21, dirty_ch02, dirty_ch12, dirty_ch22,
                            ];

                            // let diff = self.simulate_chunk(arr);

                            // for i in 0..9 {
                            //     if diff[i].len() > 0 {
                            //         let rel_ch_x = (i % 3) as i32 - 1;
                            //         let rel_ch_y = (i / 3) as i32 - 1;
                            //         self.loaded_chunks.get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap()
                            //             .apply_diff(&diff[i]);
                            //     }
                            // }

                            to_exec.push((ch_pos, arr, gr_arr, dirty_arr));
                        }
                    }
                }

                if !to_exec.is_empty() {
                    profiling::scope!("run simulation");

                    #[allow(clippy::type_complexity)]
                    let b: Vec<(
                        (i32, i32),
                        [bool; 9],
                        [Option<Rect>; 9],
                        Vec<Particle>,
                    )> = {
                        profiling::scope!("par_iter");
                        to_exec
                            .into_par_iter()
                            .map(|(ch_pos, pixels_raw, colors_raw, mut dirty_rects)| {
                                profiling::register_thread!("Simulation thread");
                                profiling::scope!("chunk");

                                let mut dirty = [false; 9];
                                let mut particles = Vec::new();
                                Simulator::simulate_chunk(
                                    ch_pos.0,
                                    ch_pos.1,
                                    pixels_raw,
                                    colors_raw,
                                    &mut dirty,
                                    &mut dirty_rects,
                                    &mut particles,
                                );

                                (ch_pos, dirty, dirty_rects, particles)
                            })
                            .collect()
                    };

                    for r in b {
                        profiling::scope!("apply");
                        let (ch_pos, dirty, dirty_rects, mut parts) = r;

                        {
                            profiling::scope!("particles");
                            world
                                .write_resource::<ParticleSystem>()
                                .active
                                .append(&mut parts);
                        }

                        for i in 0..9 {
                            let rel_ch_x = (i % 3) - 1;
                            let rel_ch_y = (i / 3) - 1;

                            if dirty[i as usize] {
                                self.loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .mark_dirty();
                            }

                            if i != 4 && dirty_rects[4].is_some() {
                                let neighbor_rect = Rect::new(
                                    if rel_ch_x == -1 {
                                        CHUNK_SIZE / 2
                                    } else {
                                        0
                                    },
                                    if rel_ch_y == -1 {
                                        CHUNK_SIZE / 2
                                    } else {
                                        0
                                    },
                                    if rel_ch_x == 0 {
                                        CHUNK_SIZE
                                    } else {
                                        CHUNK_SIZE / 2
                                    },
                                    if rel_ch_y == 0 {
                                        CHUNK_SIZE
                                    } else {
                                        CHUNK_SIZE / 2
                                    },
                                );
                                // let neighbor_rect = Rect::new(0, 0, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));
                                let mut r = self
                                    .loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .get_dirty_rect();
                                match r {
                                    Some(current) => {
                                        r = Some(current.union(neighbor_rect));
                                    },
                                    None => {
                                        r = Some(neighbor_rect);
                                    },
                                }
                                self.loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .set_dirty_rect(r);
                            }

                            if let Some(new) = dirty_rects[i as usize] {
                                let mut r = self
                                    .loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .get_dirty_rect();
                                match r {
                                    Some(current) => {
                                        r = Some(current.union(new));
                                    },
                                    None => {
                                        r = Some(new);
                                    },
                                }
                                self.loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .set_dirty_rect(r);
                            }
                        }
                    }
                }
            }
        }

        // if tick_time % 15 == 0 {
        //     let cho = self.get_chunk(0, 0);
        //     match cho {
        //         Some(ch) => {
        //             match ch.state {
        //                 ChunkState::Active => {
        //                     let ch_pos = (0, 0);
        //                     let ch00 = self.loaded_chunks.get(&chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch10 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch20 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch01 = self.loaded_chunks.get(&chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch11 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch21 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch02 = self.loaded_chunks.get(&chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch12 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch22 = self.loaded_chunks.get(&chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let arr = [
        //                         ch00, ch10, ch20,
        //                         ch01, ch11, ch21,
        //                         ch02, ch12, ch22 ];

        //                     let diff = self.simulate_chunk(arr);

        //                     for i in 0..9 {
        //                         let rel_ch_x = (i % 3) as i32 - 1;
        //                         let rel_ch_y = (i / 3) as i32 - 1;
        //                         self.loaded_chunks.get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap()
        //                             .apply_diff(&diff[i]);
        //                     }
        //                 },
        //                 _ => {},
        //             }
        //         },
        //         None => {},
        //     }
        // }
    }

    #[profiling::function]
    fn save_chunk(&mut self, index: u32) -> Result<(), Box<dyn std::error::Error>> {
        let chunk = self
            .loaded_chunks
            .get_mut(&index)
            .ok_or("Chunk not loaded")?;
        if let Some(path) = &self.path {
            if let Some(pixels) = chunk.get_pixels() {
                let chunk_path_root = path.join("chunks/");
                if !chunk_path_root.exists() {
                    std::fs::create_dir_all(&chunk_path_root).expect(
                        format!("Failed to create chunk directory @ {:?}", chunk_path_root)
                            .as_str(),
                    );
                }
                let chunk_path = chunk_path_root.join(format!(
                    "{}_{}.chunk",
                    chunk.get_chunk_x(),
                    chunk.get_chunk_y()
                ));
                let mut contents = Vec::new();

                let save = ChunkSaveFormat {
                    pixels: pixels.to_vec(),
                    colors: chunk.get_colors().to_vec(),
                };

                let pixel_data: Vec<u8> = bincode::serialize(&save)?;
                contents.extend(pixel_data);

                let r = std::fs::write(&chunk_path, contents);
                if r.is_err() {
                    log::error!(
                        "Chunk save failed @ {},{} -> {:?}",
                        chunk.get_chunk_x(),
                        chunk.get_chunk_y(),
                        chunk_path
                    );
                }
                r?;
            }
        }

        Ok(())
    }

    fn unload_all_chunks(
        &mut self,
        physics: &mut Physics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[allow(clippy::for_kv_map)] // want ? to work
        let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
        for i in keys {
            self.unload_chunk(i, physics)?;
        }
        self.loaded_chunks.clear();
        Ok(())
    }

    fn save_all_chunks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[allow(clippy::for_kv_map)] // want ? to work
        let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
        for i in keys {
            self.save_chunk(i)?;
        }
        Ok(())
    }

    #[profiling::function]
    fn queue_load_chunk(&mut self, chunk_x: i32, chunk_y: i32) -> bool {
        // make sure not loaded
        if self.is_chunk_loaded(chunk_x, chunk_y) {
            return false;
        }

        // make sure not loading
        if self
            .load_queue
            .iter()
            .any(|ch| ch.0 == chunk_x && ch.1 == chunk_y)
        {
            return false;
        }

        self.load_queue.push((chunk_x, chunk_y));

        true
    }

    // #[profiling::function]
    fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks
            .contains_key(&chunk_index(chunk_x, chunk_y))
    }

    // #[profiling::function]
    fn is_pixel_loaded(&self, x: i64, y: i64) -> bool {
        let chunk_pos = pixel_to_chunk_pos(x, y);
        self.is_chunk_loaded(chunk_pos.0, chunk_pos.1)
    }

    fn set(&mut self, x: i64, y: i64, mat: MaterialInstance) -> Result<(), String> {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(x, y);
        self.loaded_chunks
            .get_mut(&chunk_index(chunk_x, chunk_y))
            .map_or_else(
                || Err("Position is not loaded".to_string()),
                |ch| {
                    ch.set(
                        (x - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                        (y - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
                        mat,
                    )
                },
            )
    }

    fn get(&self, x: i64, y: i64) -> Result<&MaterialInstance, String> {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(x, y);
        self.loaded_chunks
            .get(&chunk_index(chunk_x, chunk_y))
            .map_or_else(
                || Err("Position is not loaded".to_string()),
                |ch| {
                    ch.get(
                        (x - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                        (y - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
                    )
                },
            )
    }

    // #[profiling::function]
    fn displace(&mut self, x: i64, y: i64, material: MaterialInstance) -> bool {
        let mut succeeded = false;

        let scan_w = 32;
        let scan_h = 32;
        let mut scan_x = 0;
        let mut scan_y = 0;
        let mut scan_delta_x = 0;
        let mut scan_delta_y = -1;
        let scan_max_i = scan_w.max(scan_h) * scan_w.max(scan_h); // the max is pointless now but could change w or h later

        for _ in 0..scan_max_i {
            if (scan_x >= -scan_w / 2)
                && (scan_x <= scan_w / 2)
                && (scan_y >= -scan_h / 2)
                && (scan_y <= scan_h / 2)
            {
                if let Ok(scan_mat) = self.get(x + i64::from(scan_x), y + i64::from(scan_y)) {
                    if scan_mat.physics == PhysicsType::Air
                        && self
                            .set(x + i64::from(scan_x), y + i64::from(scan_y), material)
                            .is_ok()
                    {
                        succeeded = true;
                        break;
                    }
                }
            }

            // update scan coordinates

            if (scan_x == scan_y)
                || ((scan_x < 0) && (scan_x == -scan_y))
                || ((scan_x > 0) && (scan_x == 1 - scan_y))
            {
                let temp = scan_delta_x;
                scan_delta_x = -scan_delta_y;
                scan_delta_y = temp;
            }

            scan_x += scan_delta_x;
            scan_y += scan_delta_y;
        }

        succeeded
    }

    fn force_update_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(ch) = self.loaded_chunks.get_mut(&chunk_index(chunk_x, chunk_y)) {
            ch.set_dirty_rect(Some(Rect::new(
                0,
                0,
                u32::from(CHUNK_SIZE),
                u32::from(CHUNK_SIZE),
            )));
        }
    }

    // #[profiling::function]
    fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&dyn Chunk> {
        self.loaded_chunks
            .get(&chunk_index(chunk_x, chunk_y))
            .map(|c| {
                // TODO: I can't figure out how to make this less stupid
                let dc: &dyn Chunk = c;
                dc
            })
    }

    // #[profiling::function]
    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_y: i32) -> Option<&mut dyn Chunk> {
        self.loaded_chunks
            .get_mut(&chunk_index(chunk_x, chunk_y))
            .map(|c| {
                // TODO: I can't figure out how to make this less stupid
                let dc: &mut dyn Chunk = c;
                dc
            })
    }

    #[profiling::function]
    fn get_zone(&self, center: (f64, f64), padding: u16) -> Rect {
        let width = self.screen_size.0 + padding * 2;
        let height = self.screen_size.1 + padding * 2;
        Rect::new(
            center.0 as i32 - i32::from(width / 2),
            center.1 as i32 - i32::from(height / 2),
            u32::from(width),
            u32::from(height),
        )
    }

    #[profiling::function]
    fn get_screen_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, 0)
    }

    #[profiling::function]
    fn get_active_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, CHUNK_SIZE)
    }

    #[profiling::function]
    fn get_load_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, CHUNK_SIZE * 5)
    }

    #[profiling::function]
    fn get_unload_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, CHUNK_SIZE * 10)
    }
}

#[derive(Default)]
pub struct PassThroughHasherU32(u32);

impl std::hash::Hasher for PassThroughHasherU32 {
    fn finish(&self) -> u64 {
        u64::from(self.0)
    }

    fn write_u32(&mut self, k: u32) {
        self.0 = k;
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("NopHasherU32 only supports u32")
    }
}

impl<'a, T: WorldGenerator + Copy + Send + Sync + 'static, C: Chunk> ChunkHandler<T, C> {
    #[profiling::function]
    pub fn new(generator: T, path: Option<PathBuf>) -> Self {
        ChunkHandler {
            loaded_chunks: HashMap::<u32, C, BuildHasherDefault<PassThroughHasherU32>>::default(),
            load_queue: vec![],
            screen_size: (1920 / 2, 1080 / 2),
            generator,
            path,
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    #[profiling::function]
    fn unload_chunk(
        &mut self,
        index: u32,
        physics: &mut Physics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chunk = self.loaded_chunks.get_mut(&index).unwrap();
        if let Some(RigidBodyState::Active(handle)) = chunk.get_rigidbody() {
            physics.remove_rigidbody(*handle);
            chunk.set_rigidbody(None);
        }

        Ok(())
    }

    #[profiling::function]
    fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        let chunk = Chunk::new_empty(chunk_x, chunk_y);
        self.loaded_chunks
            .insert(chunk_index(chunk_x, chunk_y), chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    #[test]
    fn chunk_index_correct() {
        // center
        assert_eq!(chunk_index(0, 0), 0);
        assert_eq!(chunk_index(1, 0), 3);
        assert_eq!(chunk_index(0, 1), 5);
        assert_eq!(chunk_index(1, 1), 12);
        assert_eq!(chunk_index(-1, 0), 1);
        assert_eq!(chunk_index(0, -1), 2);
        assert_eq!(chunk_index(-1, -1), 4);
        assert_eq!(chunk_index(1, -1), 7);
        assert_eq!(chunk_index(-1, 1), 8);

        // some random nearby ones
        assert_eq!(chunk_index(207, 432), 818_145);
        assert_eq!(chunk_index(285, -65), 244_779);
        assert_eq!(chunk_index(958, 345), 3_397_611);
        assert_eq!(chunk_index(632, 255), 1_574_935);
        assert_eq!(chunk_index(-942, 555), 4_481_631);
        assert_eq!(chunk_index(696, 589), 3_304_913);
        assert_eq!(chunk_index(-201, -623), 1_356_726);
        assert_eq!(chunk_index(741, 283), 2_098_742);
        assert_eq!(chunk_index(-302, 718), 2_081_216);
        assert_eq!(chunk_index(493, 116), 742_603);

        // some random far ones
        assert_eq!(chunk_index(1258, 7620), 157_661_886);
        assert_eq!(chunk_index(9438, 4645), 396_685_151);
        assert_eq!(chunk_index(6852, -7129), 390_936_998);
        assert_eq!(chunk_index(-7692, -912), 148_033_644);
        assert_eq!(chunk_index(-4803, -131), 48_674_172);
        assert_eq!(chunk_index(-4565, 8366), 334_425_323);
        assert_eq!(chunk_index(248, -126), 279_629);
        assert_eq!(chunk_index(-1125, 3179), 37_050_886);
        assert_eq!(chunk_index(4315, -4044), 139_745_490);
        assert_eq!(chunk_index(-3126, 9730), 330_560_076);

        // maximum
        assert_eq!(chunk_index(-27804, 18537), u32::MAX);
    }

    #[test]
    fn chunk_index_inv_correct() {
        // center
        assert_eq!(chunk_index_inv(0), (0, 0));
        assert_eq!(chunk_index_inv(3), (1, 0));
        assert_eq!(chunk_index_inv(5), (0, 1));
        assert_eq!(chunk_index_inv(12), (1, 1));
        assert_eq!(chunk_index_inv(1), (-1, 0));
        assert_eq!(chunk_index_inv(2), (0, -1));
        assert_eq!(chunk_index_inv(4), (-1, -1));
        assert_eq!(chunk_index_inv(7), (1, -1));
        assert_eq!(chunk_index_inv(8), (-1, 1));

        // some random nearby ones
        assert_eq!(chunk_index_inv(818_145), (207, 432));
        assert_eq!(chunk_index_inv(244_779), (285, -65));
        assert_eq!(chunk_index_inv(3_397_611), (958, 345));
        assert_eq!(chunk_index_inv(1_574_935), (632, 255));
        assert_eq!(chunk_index_inv(4_481_631), (-942, 555));
        assert_eq!(chunk_index_inv(3_304_913), (696, 589));
        assert_eq!(chunk_index_inv(1_356_726), (-201, -623));
        assert_eq!(chunk_index_inv(2_098_742), (741, 283));
        assert_eq!(chunk_index_inv(2_081_216), (-302, 718));
        assert_eq!(chunk_index_inv(742_603), (493, 116));

        // some random far ones
        assert_eq!(chunk_index_inv(157_661_886), (1258, 7620));
        assert_eq!(chunk_index_inv(396_685_151), (9438, 4645));
        assert_eq!(chunk_index_inv(390_936_998), (6852, -7129));
        assert_eq!(chunk_index_inv(148_033_644), (-7692, -912));
        assert_eq!(chunk_index_inv(48_674_172), (-4803, -131));
        assert_eq!(chunk_index_inv(334_425_323), (-4565, 8366));
        assert_eq!(chunk_index_inv(279_629), (248, -126));
        assert_eq!(chunk_index_inv(37_050_886), (-1125, 3179));
        assert_eq!(chunk_index_inv(139_745_490), (4315, -4044));
        assert_eq!(chunk_index_inv(330_560_076), (-3126, 9730));

        // maximum
        assert_eq!(chunk_index_inv(u32::MAX), (-27804, 18537));
    }

    #[test]
    fn chunk_index_correctly_invertible() {
        for _ in 0..1000 {
            let x: i32 = rand::thread_rng().gen_range(-10000..10000);
            let y: i32 = rand::thread_rng().gen_range(-10000..10000);

            println!("Testing ({}, {})...", x, y);
            let index = chunk_index(x, y);
            let result = chunk_index_inv(index);

            assert_eq!(result, (x, y));
        }
    }

    #[test]
    fn chunk_update_order() {
        for _ in 0..100 {
            let x: i32 = rand::thread_rng().gen_range(-10000..10000);
            let y: i32 = rand::thread_rng().gen_range(-10000..10000);

            println!("Testing ({}, {})...", x, y);

            let my_order = super::chunk_update_order(x, y);

            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx != 0 || dy != 0 {
                        // surrounding chunks should not be able to update at the same time
                        assert_ne!(super::chunk_update_order(x + dx, y + dy), my_order);
                    }
                }
            }
        }
    }
}

// #[profiling::function]
#[inline]
pub fn pixel_to_chunk_pos(x: i64, y: i64) -> (i32, i32) {
    (
        (x as f64 / f64::from(CHUNK_SIZE)).floor() as i32,
        (y as f64 / f64::from(CHUNK_SIZE)).floor() as i32,
    )
}

#[inline]
pub fn pixel_to_chunk_pos_with_chunk_size(x: i64, y: i64, chunk_size: u16) -> (i32, i32) {
    (
        (x as f64 / f64::from(chunk_size)).floor() as i32,
        (y as f64 / f64::from(chunk_size)).floor() as i32,
    )
}

#[inline]
pub fn chunk_index(chunk_x: i32, chunk_y: i32) -> u32 {
    fn int_to_nat(i: i32) -> u32 {
        if i >= 0 {
            (2 * i) as u32
        } else {
            (-2 * i - 1) as u32
        }
    }
    let xx: u32 = int_to_nat(chunk_x);
    let yy: u32 = int_to_nat(chunk_y);

    // TODO: this multiply is the first thing to overflow if you go out too far
    //          (though you need to go out ~32768 chunks (2^16 / 2)
    ((u64::from(xx + yy) * u64::from(xx + yy + 1)) / 2 + u64::from(yy)) as u32
}

#[inline]
pub fn chunk_index_inv(index: u32) -> (i32, i32) {
    let w = (((8 * u64::from(index) + 1) as f64).sqrt() - 1.0).floor() as u64 / 2;
    let t = (w * w + w) / 2;
    let yy = u64::from(index) - t;
    let xx = w - yy;
    fn nat_to_int(i: u64) -> i32 {
        if i % 2 == 0 {
            (i / 2) as i32
        } else {
            -((i / 2 + 1) as i32)
        }
    }
    let x = nat_to_int(xx);
    let y = nat_to_int(yy);

    (x, y)
}

pub fn chunk_update_order(chunk_x: i32, chunk_y: i32) -> u8 {
    let yy = (-chunk_y).rem_euclid(2) as u8;
    let xx = chunk_x.rem_euclid(2) as u8;

    yy * 2 + xx
}
