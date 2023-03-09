use crate::game::common::hashmap_ext::HashMapExt;
use crate::game::common::world::gen::populator::ChunkContext;
use crate::game::common::world::gen::structure::UpdateStructureNodes;
use crate::game::common::world::particle::ParticleSystem;
use crate::game::common::world::simulator::{Simulator, SimulatorChunkContext};
use crate::game::common::world::{Loader, Position};
use crate::game::common::Registries;
use crate::game::common::{Rect, Settings};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::hash::BuildHasherDefault;
use std::path::PathBuf;
use std::sync::Arc;

use futures::channel::oneshot::Receiver;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rapier2d::prelude::{Collider, RigidBody, RigidBodyHandle};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use specs::{Join, ReadStorage, RunNow, WorldExt};

use super::gen::WorldGenerator;
use super::material::{color::Color, PhysicsType};
use super::mesh::Mesh;
use super::particle::Particle;
use super::physics::Physics;
use crate::game::common::world::material::MaterialInstance;

pub const CHUNK_SIZE: u16 = 100;
// must be a factor of CHUNK_SIZE
// also (CHUNK_SIZE / LIGHT_SCALE)^2 must be <= 1024 for compute shader (and local_size needs to be set to CHUNK_SIZE / LIGHT_SCALE in the shader)
pub const LIGHT_SCALE: u8 = 4;

#[warn(clippy::large_enum_variant)]
pub enum RigidBodyState {
    Active(RigidBodyHandle),
    Inactive(Box<RigidBody>, Vec<Collider>),
}

pub trait Chunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self
    where
        Self: Sized;

    fn chunk_x(&self) -> i32;
    fn chunk_y(&self) -> i32;

    fn state(&self) -> ChunkState;
    fn set_state(&mut self, state: ChunkState);

    fn dirty_rect(&self) -> Option<Rect<i32>>;
    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>);

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>);
    fn pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>;
    fn pixels(&self) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>;
    fn set_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    );
    fn colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];
    fn colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];
    fn lights_mut(&mut self) -> &mut [[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize];
    fn lights(&self) -> &[[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize];
    fn set_background_pixels(
        &mut self,
        pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    );
    fn background_pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>;
    fn background_pixels(
        &self,
    ) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>;
    fn set_background_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    );
    fn background_colors_mut(&mut self)
        -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];
    fn background_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4];

    fn generate_mesh(&mut self) -> Result<(), String>;
    // fn get_tris(&self) -> &Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>>;
    fn mesh_loops(&self) -> &Option<Mesh>;
    fn rigidbody(&self) -> &Option<RigidBodyState>;
    fn rigidbody_mut(&mut self) -> &mut Option<RigidBodyState>;
    fn set_rigidbody(&mut self, body: Option<RigidBodyState>);

    fn mark_dirty(&mut self);

    fn refresh(&mut self);

    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn set_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance);

    fn pixel(&self, x: u16, y: u16) -> Result<&MaterialInstance, String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn pixel_unchecked(&self, x: u16, y: u16) -> &MaterialInstance;

    fn replace_pixel<F>(&mut self, x: u16, y: u16, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>;

    fn set_light(&mut self, x: u16, y: u16, light: [f32; 3]) -> Result<(), String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn set_light_unchecked(&mut self, x: u16, y: u16, light: [f32; 3]);

    fn light(&self, x: u16, y: u16) -> Result<&[f32; 3], String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn light_unchecked(&self, x: u16, y: u16) -> &[f32; 3];

    fn set_color(&mut self, x: u16, y: u16, color: Color) -> Result<(), String>;
    fn color(&self, x: u16, y: u16) -> Result<Color, String>;

    fn set_background(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn set_background_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance);

    fn background(&self, x: u16, y: u16) -> Result<&MaterialInstance, String>;
    /// # Safety
    /// x and y must be in `0..CHUNK_SIZE`
    unsafe fn background_unchecked(&self, x: u16, y: u16) -> &MaterialInstance;

    #[profiling::function]
    fn apply_diff(&mut self, diff: &[(u16, u16, MaterialInstance)]) {
        for (x, y, mat) in diff {
            self.set(*x, *y, mat.clone()).unwrap(); // TODO: handle this Err
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    NotGenerated,
    Generating(u8), // stage
    Cached,
    Active,
}

#[derive(Debug)]
pub struct ChunkHandler<C: Chunk> {
    pub loaded_chunks: HashMap<u32, C, BuildHasherDefault<PassThroughHasherU32>>,
    pub load_queue: Vec<(i32, i32)>,
    pub gen_pool: rayon::ThreadPool,
    pub gen_threads: Vec<(u32, Receiver<ChunkGenOutput>)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: Arc<dyn WorldGenerator>,
    pub path: Option<PathBuf>,
}

#[allow(clippy::cast_lossless)]
pub type ChunkGenOutput = (
    u32,
    Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    Box<[u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]>,
    Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    Box<[u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]>,
);

pub trait ChunkHandlerGeneric {
    fn tick(
        &mut self,
        tick_time: u32,
        settings: &Settings,
        world: &mut specs::World,
        physics: &mut Physics,
        registries: Arc<Registries>,
        seed: i32,
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
    fn replace<F>(&mut self, x: i64, y: i64, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>;
    fn displace(&mut self, x: i64, y: i64, material: MaterialInstance) -> bool;
    fn force_update_chunk(&mut self, chunk_x: i32, chunk_y: i32);
    fn get_zone(&self, center: (f64, f64), padding: u16) -> Rect<i32>;
    fn get_screen_zone(&self, center: (f64, f64)) -> Rect<i32>;
    fn get_active_zone(&self, center: (f64, f64)) -> Rect<i32>;
    fn get_load_zone(&self, center: (f64, f64)) -> Rect<i32>;
    fn get_unload_zone(&self, center: (f64, f64)) -> Rect<i32>;
}

#[derive(Serialize, Deserialize)]
struct ChunkSaveFormat {
    pixels: Vec<MaterialInstance>,
    colors: Vec<u8>,
}

impl<C: Chunk + Send> ChunkHandlerGeneric for ChunkHandler<C> {
    // #[profiling::function] // breaks clippy
    #[allow(clippy::too_many_lines)]
    fn tick(
        &mut self,
        tick_time: u32,
        settings: &Settings,
        world: &mut specs::World,
        physics: &mut Physics,
        registries: Arc<Registries>,
        seed: i32,
    ) {
        profiling::scope!("tick");

        let (loaders, positions) =
            world.system_data::<(ReadStorage<Loader>, ReadStorage<Position>)>();

        let unload_zone: Vec<Rect<i32>> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_unload_zone((p.x, p.y)))
            .collect();
        let load_zone: Vec<Rect<i32>> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_load_zone((p.x, p.y)))
            .collect();
        let active_zone: Vec<Rect<i32>> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_active_zone((p.x, p.y)))
            .collect();
        let _screen_zone: Vec<Rect<i32>> = (&loaders, &positions)
            .join()
            .map(|(_, p)| self.get_screen_zone((p.x, p.y)))
            .collect();

        if settings.load_chunks {
            {
                profiling::scope!("queue chunk loading");
                for load_zone in load_zone {
                    for px in load_zone.range_lr().step_by(CHUNK_SIZE.into()) {
                        for py in load_zone.range_tb().step_by(CHUNK_SIZE.into()) {
                            let chunk_pos = pixel_to_chunk_pos(px.into(), py.into());
                            self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
                        }
                    }
                }
            }

            {
                profiling::scope!("chunk loading");

                self.load_queue.sort_by(|(a_x, a_y), (b_x, b_y)| {
                    let c1_x = a_x * i32::from(CHUNK_SIZE);
                    let c1_y = a_y * i32::from(CHUNK_SIZE);
                    let c2_x = b_x * i32::from(CHUNK_SIZE);
                    let c2_y = b_y * i32::from(CHUNK_SIZE);

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

                    d2.cmp(&d1)
                });

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

                let state = self.loaded_chunks.get(&key).unwrap().state(); // copy
                let rect = Rect::new_wh(
                    self.loaded_chunks.get(&key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                    self.loaded_chunks.get(&key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                    CHUNK_SIZE,
                    CHUNK_SIZE,
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
                            }
                            if let Err(e) = self.unload_chunk(key, physics) {
                                log::error!(
                                    "Chunk @ {}, {} failed to unload: {:?}",
                                    chunk_index_inv(key).0,
                                    chunk_index_inv(key).1,
                                    e
                                );
                            }
                            keep_map[i] = false;
                        } else if active_zone.iter().any(|z| rect.intersects(z)) {
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().chunk_x();
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().chunk_y();
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

                                let state = ch.unwrap().state();

                                matches!(state, ChunkState::Cached | ChunkState::Active)
                            }) {
                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_state(ChunkState::Active);
                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_dirty_rect(Some(Rect::new_wh(
                                        0, 0, CHUNK_SIZE, CHUNK_SIZE,
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

        if settings.load_chunks && tick_time % 2 == 0 {
            let num_active = self
                .loaded_chunks
                .values()
                .filter(|c| c.state() == ChunkState::Active)
                .count();
            let num_cached = self
                .loaded_chunks
                .values()
                .filter(|c| c.state() == ChunkState::Cached)
                .count();

            // generate new chunks
            {
                profiling::scope!("chunk update B");

                // get keys for all chunks sorted by distance to nearest loader
                let mut keys = self
                    .loaded_chunks
                    .iter()
                    .filter_map(|(k, c)| {
                        if c.state() == ChunkState::NotGenerated {
                            Some(k)
                        } else {
                            None
                        }
                    })
                    .copied()
                    .collect::<Vec<u32>>();
                if !loaders.is_empty() {
                    profiling::scope!("sort");
                    keys.sort_by(|a, b| {
                        let a = self.loaded_chunks.get(a).unwrap();
                        let b = self.loaded_chunks.get(b).unwrap();
                        let c1_x = a.chunk_x() * i32::from(CHUNK_SIZE);
                        let c1_y = a.chunk_y() * i32::from(CHUNK_SIZE);
                        let c2_x = b.chunk_x() * i32::from(CHUNK_SIZE);
                        let c2_y = b.chunk_y() * i32::from(CHUNK_SIZE);

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

                let mut num_loaded_this_tick = 0;

                // list of chunks that need to be generated
                // u32 is key, i32s are chunk x and y
                let to_generate = keys.iter().filter_map(|key| {
                    let rect = Rect::new_wh(
                        self.loaded_chunks.get(key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                        self.loaded_chunks.get(key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );

                    // keys are filtered by state == NotGenerated already
                    assert!(self.loaded_chunks.get(key).unwrap().state() == ChunkState::NotGenerated);

                    // start generating chunks waiting to generate
                    if unload_zone.iter().any(|z| rect.intersects(z)) && num_loaded_this_tick < 32 {
                        let chunk_x = self.loaded_chunks.get_mut(key).unwrap().chunk_x();
                        let chunk_y = self.loaded_chunks.get_mut(key).unwrap().chunk_y();

                        let mut should_generate = true;

                        // skip if already generating this chunk
                        if self.gen_threads.iter().any(|(k, _)| k == key) {
                            should_generate = false;
                        }

                        // try to load from file
                        if let Some(path) = &self.path {
                            let chunk_path_root = path.join("chunks/");
                            if !chunk_path_root.exists() {
                                std::fs::create_dir_all(&chunk_path_root).expect(
                                    format!(
                                        "Failed to create chunk directory @ {chunk_path_root:?}"
                                    )
                                    .as_str(),
                                );
                            }
                            let chunk_path =
                                chunk_path_root.join(format!("{chunk_x}_{chunk_y}.chunk"));
                            if chunk_path.exists() {
                                if let Ok(data) = std::fs::read(&chunk_path) {
                                    match bincode::deserialize(&data) {
                                        Ok(res) => {
                                            let save: ChunkSaveFormat = res;

                                            if save.pixels.len()
                                                == (CHUNK_SIZE as usize * CHUNK_SIZE as usize)
                                            {
                                                let chunk =  self.loaded_chunks.get_mut(key).unwrap();
                                                chunk.set_state(ChunkState::Cached);
                                                chunk.set_pixels(save.pixels.try_into().unwrap());
                                                chunk.mark_dirty();
                                                let _ = chunk.generate_mesh();

                                                if save.colors.len()
                                                    == (CHUNK_SIZE as usize
                                                        * CHUNK_SIZE as usize
                                                        * 4)
                                                {
                                                    chunk.set_pixel_colors(save.colors.try_into().unwrap());
                                                } else {
                                                    log::error!("colors Vec is the wrong size: {} (expected {})", save.colors.len(), CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4);
                                                    chunk.refresh();
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
                            num_loaded_this_tick += 1;
                            return Some((*key, chunk_x, chunk_y));
                        }
                    }

                    None
                }).collect::<Vec<_>>();

                // spawn chunk generation tasks
                {
                    profiling::scope!("gen chunks");
                    for (key, chunk_x, chunk_y) in to_generate {
                        let generator = self.generator.clone();
                        let reg = registries.clone();
                        let (tx, rx) = futures::channel::oneshot::channel();
                        self.gen_pool.spawn_fifo(move || {
                            profiling::register_thread!("Generation thread");
                            profiling::scope!("chunk");

                            // these arrays are too large for the stack

                            let mut pixels = Box::new(
                                [(); (CHUNK_SIZE * CHUNK_SIZE) as usize]
                                    .map(|_| MaterialInstance::air()),
                            );

                            #[allow(clippy::cast_lossless)]
                            let mut colors =
                                Box::new([0; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]);

                            let mut background = Box::new(
                                [(); (CHUNK_SIZE * CHUNK_SIZE) as usize]
                                    .map(|_| MaterialInstance::air()),
                            );

                            #[allow(clippy::cast_lossless)]
                            let mut background_colors =
                                Box::new([0; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]);

                            generator.generate(
                                chunk_x,
                                chunk_y,
                                seed,
                                &mut pixels,
                                &mut colors,
                                &mut background,
                                &mut background_colors,
                                reg.as_ref(),
                            );

                            tx.send((key, pixels, colors, background, background_colors))
                                .unwrap();
                        });

                        self.gen_threads.push((key, rx));
                    }
                }

                // get data from a number of finished generation tasks
                let mut generated = vec![];
                self.gen_threads.retain_mut(|(_, v)| {
                    if generated.len() < if num_cached + num_active < 4 { 32 } else { 8 } {
                        if let Ok(Some(g)) = v.try_recv() {
                            generated.push(g);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                });

                // put generated data into chunk
                let keys: Vec<_> = generated
                    .into_iter()
                    .filter_map(|(key, pixels, colors, background, background_colors)| {
                        profiling::scope!("finish chunk");

                        self.loaded_chunks.get_mut(&key).map(|chunk| {
                            chunk.set_state(ChunkState::Generating(0));
                            chunk.set_pixels(pixels);
                            chunk.set_pixel_colors(colors);
                            chunk.set_background_pixels(background);
                            chunk.set_background_pixel_colors(background_colors);
                            key
                        })
                    })
                    .collect();

                // do population stage 0
                {
                    profiling::scope!("populate stage 0");
                    let pops = self.generator.populators();
                    self.loaded_chunks
                        .get_many_var_mut(&keys)
                        .unwrap()
                        .into_par_iter()
                        .for_each(|chunk| {
                            profiling::scope!("populate thread");
                            pops.populate(
                                0,
                                &mut [chunk as &mut dyn Chunk],
                                seed,
                                registries.as_ref(),
                            );
                        });
                }
            }

            drop(loaders);
            drop(positions);

            // unloading NotGenerated or Generating chunks
            // populate chunks
            {
                profiling::scope!("chunk update C");

                let mut keep_map = vec![true; self.loaded_chunks.len()];
                let keys = self.loaded_chunks.keys().copied().collect::<Vec<u32>>();
                let mut populated_num = 0;
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().state(); // copy
                    let rect = Rect::new_wh(
                        self.loaded_chunks.get(&key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                        self.loaded_chunks.get(&key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
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
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().chunk_x();
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().chunk_y();

                            let max_stage = self.generator.max_gen_stage();

                            if cur_stage >= max_stage {
                                let _ = self.loaded_chunks.get_mut(&key).unwrap().generate_mesh();

                                self.loaded_chunks
                                    .get_mut(&key)
                                    .unwrap()
                                    .set_state(ChunkState::Cached);
                            } else {
                                if populated_num
                                    < if num_active < 16 {
                                        32
                                    } else if num_active < 64 {
                                        16
                                    } else {
                                        8
                                    }
                                    && [
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
                                        let Some(chunk) = ch else {
                                            return false;
                                        };

                                        if chunk.pixels().is_none() {
                                            return false;
                                        }

                                        let state = ch.unwrap().state();

                                        match state {
                                            ChunkState::Cached | ChunkState::Active => true,
                                            ChunkState::Generating(st) if st >= cur_stage => true,
                                            _ => false,
                                        }
                                    })
                                {
                                    let mut keys = Vec::new();

                                    let range = i32::from(cur_stage + 1);

                                    // try to gather the nearby chunks needed to populate this one
                                    for y in -range..=range {
                                        for x in -range..=range {
                                            keys.push(chunk_index(chunk_x + x, chunk_y + y));
                                        }
                                    }

                                    let chunks = self.loaded_chunks.get_many_var_mut(&keys);

                                    // if we failed to get all nearby chunks, don't populate and don't go to the next stage
                                    if let Some((true, chunks)) = chunks
                                        .map(|chs| (chs.iter().all(|c| c.pixels().is_some()), chs))
                                    {
                                        let mut chunks_dyn: Vec<_> = chunks
                                            .into_iter()
                                            .map(|c| c as &mut dyn Chunk)
                                            .collect();

                                        if cur_stage + 1 == 1 {
                                            let mut ctx =
                                                ChunkContext::<1>::new(&mut chunks_dyn).unwrap();
                                            let mut rng = StdRng::seed_from_u64(
                                                seed as u64
                                                    + u64::from(chunk_index(
                                                        ctx.center_chunk().0,
                                                        ctx.center_chunk().1,
                                                    )),
                                            );
                                            for feat in self.generator.features() {
                                                feat.generate(
                                                    &mut ctx,
                                                    seed,
                                                    &mut rng,
                                                    registries.as_ref(),
                                                    world,
                                                );
                                                world.maintain();
                                            }
                                        }

                                        self.generator.populators().populate(
                                            cur_stage + 1,
                                            &mut chunks_dyn,
                                            seed,
                                            registries.as_ref(),
                                        );

                                        self.loaded_chunks
                                            .get_mut(&key)
                                            .unwrap()
                                            .set_state(ChunkState::Generating(cur_stage + 1));

                                        populated_num += 1;
                                    }
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

                // tick structures
                let mut update_structures = UpdateStructureNodes {
                    chunk_handler: self,
                    registries: registries.clone(),
                };
                update_structures.run_now(world);
                world.maintain();

                let mut iter = keep_map.iter();
                self.loaded_chunks.retain(|_, _| *iter.next().unwrap());
            }
        }

        if settings.simulate_chunks {
            profiling::scope!("chunk simulate");

            let old_dirty_rects = self
                .loaded_chunks
                .iter_mut()
                .map(|(key, ch)| {
                    let rect = ch.dirty_rect();
                    ch.set_dirty_rect(None);
                    (*key, rect)
                })
                .collect::<HashMap<_, _>>();

            let keys = self.loaded_chunks.keys().copied().collect::<Vec<_>>();
            for tick_phase in 0..4 {
                profiling::scope!("phase", format!("phase {}", tick_phase).as_str());
                let mut to_exec = vec![];
                for key in &keys {
                    let ch = self.loaded_chunks.get(key).unwrap();
                    let state = ch.state(); // copy
                    let ch_pos = (ch.chunk_x(), ch.chunk_y());

                    if chunk_update_order(ch_pos.0, ch_pos.1) == tick_phase
                        && state == ChunkState::Active
                    {
                        profiling::scope!("iter");

                        if old_dirty_rects.get(key).is_some() {
                            // SAFETY: the same chunks' arrays may be modified mutably on multiple threads at once, which is necessary for multithreading
                            // However, ticking a chunk can only affect pixels within CHUNK_SIZE/2 of the center chunk (this is unchecked)
                            //   and we the 4-phase thing ensures no chunks directly next to each other are ticked at the same time
                            //   so multiple threads will not modify the same index in the array at the same time
                            // The chunk arrays are cast to `&[UnsafeCell<T>; _]`, so there should be no actual `&mut`s involved
                            // There is still a very good chance there's UB here, I'm not an expert on aliasing
                            // TODO: see if miri can run this

                            let Some(arr) = [
                                (-1, -1),
                                (0, -1),
                                (1, -1),
                                (-1, 0),
                                (0, 0),
                                (1, 0),
                                (-1, 1),
                                (0, 1),
                                (1, 1),
                            ]
                                .into_iter()
                                .map(|(x, y)| {
                                    let chunk = self.loaded_chunks.get_mut(&chunk_index(ch_pos.0 + x, ch_pos.1 + y));
                                    chunk.and_then(|c| {
                                        c.pixels_mut().as_mut().map(|raw| {
                                            // blatantly bypassing the borrow checker, see safety comment above
                                            unsafe { &*(raw.as_mut() as *mut _ as *const _) }
                                        }).map(|pixels| {
                                            // blatantly bypassing the borrow checker, see safety comment above
                                            // I'm not sure if doing this while the data is already in a `&[UnsafeCell<_>; _]` is UB

                                            let raw: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = c.colors_mut();
                                            let colors = unsafe { &*(raw as *const [UnsafeCell<u8>; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)]) };

                                            let raw: *mut [[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize] = c.lights_mut();
                                            let lights = unsafe { &*(raw as *const [UnsafeCell<[f32; 4]>; CHUNK_SIZE as usize * CHUNK_SIZE as usize]) };

                                            let dirty_rect = *old_dirty_rects
                                                .get(&chunk_index(ch_pos.0 + x, ch_pos.1 + y))
                                                .unwrap();

                                            let colors = unsafe {
                                                // Safety: `Color` has is exactly identical to 4 `u8`s (statically asserted)
                                                //         so it's fine to cast a slice of `u8` to a slice of `Color` with 1/4 the size
                                                let cs: &[UnsafeCell<Color>] = core::slice::from_raw_parts(colors.as_ptr().cast(), colors.len()/4);
                                                cs.try_into().unwrap_unchecked()
                                            };

                                            SimulatorChunkContext {
                                                pixels,
                                                colors,
                                                lights,
                                                dirty: false,
                                                dirty_rect,
                                            }
                                        })
                                    })
                                })
                                .collect::<Option<Vec<_>>>()
                                .map(|v| v.try_into().unwrap())
                            else {
                                continue;
                            };

                            to_exec.push((ch_pos, arr));
                        }
                    }
                }

                if !to_exec.is_empty() {
                    profiling::scope!("run simulation");

                    #[allow(clippy::type_complexity)]
                    let b: Vec<(
                        (i32, i32),
                        [(bool, Option<Rect<i32>>); 9],
                        Vec<Particle>,
                    )> = {
                        profiling::scope!("par_iter");
                        let reg = registries.clone();
                        to_exec
                            .into_par_iter()
                            .map(move |(ch_pos, mut chunk_data)| {
                                profiling::register_thread!("Simulation thread");
                                profiling::scope!("chunk");

                                let mut particles = Vec::new();
                                Simulator::simulate_chunk(
                                    ch_pos.0,
                                    ch_pos.1,
                                    &mut chunk_data,
                                    &mut particles,
                                    reg.clone(),
                                );

                                let dirty_info = chunk_data.map(|d| (d.dirty, d.dirty_rect));
                                (ch_pos, dirty_info, particles)
                            })
                            .collect()
                    };

                    for r in b {
                        profiling::scope!("apply");
                        let (ch_pos, dirty_info, mut parts) = r;

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

                            if dirty_info[i as usize].0 {
                                self.loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .mark_dirty();
                            }

                            if i != 4 && dirty_info[4].1.is_some() {
                                let neighbor_rect = Rect::new_wh(
                                    if rel_ch_x == -1 { CHUNK_SIZE / 2 } else { 0 },
                                    if rel_ch_y == -1 { CHUNK_SIZE / 2 } else { 0 },
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
                                // let neighbor_rect = Rect::new_wh(0, 0, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));
                                let mut r = self
                                    .loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .dirty_rect();
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

                            if let Some(new) = dirty_info[i as usize].1 {
                                let mut r = self
                                    .loaded_chunks
                                    .get_mut(&chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                                    .unwrap()
                                    .dirty_rect();
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
    }

    #[profiling::function]
    fn save_chunk(&mut self, index: u32) -> Result<(), Box<dyn std::error::Error>> {
        let chunk = self
            .loaded_chunks
            .get_mut(&index)
            .ok_or("Chunk not loaded")?;
        if let Some(path) = &self.path {
            if let Some(pixels) = chunk.pixels() {
                let chunk_path_root = path.join("chunks/");
                if !chunk_path_root.exists() {
                    std::fs::create_dir_all(&chunk_path_root).expect(
                        format!("Failed to create chunk directory @ {chunk_path_root:?}").as_str(),
                    );
                }
                let chunk_path =
                    chunk_path_root.join(format!("{}_{}.chunk", chunk.chunk_x(), chunk.chunk_y()));
                let mut contents = Vec::new();

                let save = ChunkSaveFormat {
                    pixels: pixels.to_vec(),
                    colors: chunk.colors().to_vec(),
                };

                let pixel_data: Vec<u8> = bincode::serialize(&save)?;
                contents.extend(pixel_data);

                let r = std::fs::write(&chunk_path, contents);
                if r.is_err() {
                    log::error!(
                        "Chunk save failed @ {},{} -> {:?}",
                        chunk.chunk_x(),
                        chunk.chunk_y(),
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
                    ch.pixel(
                        (x - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                        (y - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
                    )
                },
            )
    }

    fn replace<F>(&mut self, x: i64, y: i64, cb: F) -> Result<bool, String>
    where
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(x, y);
        self.loaded_chunks
            .get_mut(&chunk_index(chunk_x, chunk_y))
            .map_or_else(
                || Err("Position is not loaded".to_string()),
                |ch| {
                    ch.replace_pixel(
                        (x - i64::from(chunk_x) * i64::from(CHUNK_SIZE)) as u16,
                        (y - i64::from(chunk_y) * i64::from(CHUNK_SIZE)) as u16,
                        cb,
                    )
                },
            )
    }

    #[profiling::function]
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
                if let Ok(true) =
                    self.replace(x + i64::from(scan_x), y + i64::from(scan_y), |scan_mat| {
                        (scan_mat.physics == PhysicsType::Air).then_some(material.clone())
                    })
                {
                    succeeded = true;
                    break;
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
            ch.set_dirty_rect(Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE)));
        }
    }

    // #[profiling::function]
    fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&dyn Chunk> {
        self.loaded_chunks
            .get(&chunk_index(chunk_x, chunk_y))
            .map(|c| c as _)
    }

    // #[profiling::function]
    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_y: i32) -> Option<&mut dyn Chunk> {
        self.loaded_chunks
            .get_mut(&chunk_index(chunk_x, chunk_y))
            .map(|c| c as _)
    }

    #[profiling::function]
    fn get_zone(&self, center: (f64, f64), padding: u16) -> Rect<i32> {
        let width = self.screen_size.0 + padding * 2;
        let height = self.screen_size.1 + padding * 2;
        Rect::new_wh(
            center.0 as i32 - i32::from(width / 2),
            center.1 as i32 - i32::from(height / 2),
            width,
            height,
        )
    }

    #[profiling::function]
    fn get_screen_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, 0)
    }

    #[profiling::function]
    fn get_active_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE)
    }

    #[profiling::function]
    fn get_load_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE * 10)
    }

    #[profiling::function]
    fn get_unload_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE * 15)
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

impl<C: Chunk> ChunkHandler<C> {
    #[profiling::function]
    pub fn new(generator: impl WorldGenerator + 'static, path: Option<PathBuf>) -> Self {
        ChunkHandler {
            loaded_chunks: HashMap::with_capacity_and_hasher(1000, BuildHasherDefault::default()),
            load_queue: vec![],
            gen_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .expect("Failed to build gen_poool"),
            gen_threads: vec![],
            screen_size: (1920 / 2, 1080 / 2),
            generator: Arc::new(generator),
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
        if let Some(RigidBodyState::Active(handle)) = chunk.rigidbody() {
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

            println!("Testing ({x}, {y})...");
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

            println!("Testing ({x}, {y})...");

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
pub const fn pixel_to_chunk_pos(x: i64, y: i64) -> (i32, i32) {
    // div_euclid is the same as div_floor in this case (div_floor is currenlty unstable)
    (
        x.div_euclid(CHUNK_SIZE as _) as _,
        y.div_euclid(CHUNK_SIZE as _) as _,
    )
}

#[inline]
pub const fn pixel_to_chunk_pos_with_chunk_size(x: i64, y: i64, chunk_size: u16) -> (i32, i32) {
    // div_euclid is the same as div_floor in this case (div_floor is currenlty unstable)
    (
        x.div_euclid(chunk_size as _) as _,
        y.div_euclid(chunk_size as _) as _,
    )
}

#[inline]
pub fn chunk_index(chunk_x: i32, chunk_y: i32) -> u32 {
    #[inline]
    const fn int_to_nat(i: i32) -> u32 {
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
    const fn nat_to_int(i: u64) -> i32 {
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

pub const fn chunk_update_order(chunk_x: i32, chunk_y: i32) -> u8 {
    let yy = (-chunk_y).rem_euclid(2) as u8;
    let xx = chunk_x.rem_euclid(2) as u8;

    yy * 2 + xx
}
