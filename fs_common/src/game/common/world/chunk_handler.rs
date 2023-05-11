use std::{cell::UnsafeCell, fmt::Debug, path::PathBuf, sync::Arc};

use asefile::AsepriteFile;
use chunksystem::{ChunkKey, ChunkManager, ChunkQuery};
use futures::channel::oneshot::Receiver;
use rand::{rngs::StdRng, SeedableRng};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use specs::{Join, ReadStorage, RunNow, WorldExt};

use crate::game::common::{
    hashmap_ext::HashMapExt,
    world::{
        chunk_index, chunk_update_order,
        gen::{populator::ChunkContext, structure::UpdateStructureNodes, GenBuffers, GenContext},
        material::buf::MaterialRect,
        particle::{Particle, ParticleSystem},
        pixel_to_chunk_pos,
        simulator::{Simulator, SimulatorChunkContext},
        tile_entity::{TileEntityCommon, TileEntityTickContext},
        ChunkState, Loader, Position, CHUNK_SIZE,
    },
    FileHelper, Rect, Registries, Settings,
};

use super::{
    chunk_data::SidedChunkData,
    gen::WorldGenerator,
    material::{color::Color, MaterialInstance},
    physics::Physics,
    tile_entity::TileEntitySided,
    Chunk, ChunkRigidBodyState, SidedChunk, CHUNK_AREA,
};

pub struct ChunkHandler<C: Chunk> {
    pub manager: ChunkManager<C>,
    pub load_queue: Vec<(i32, i32)>,
    pub gen_pool: rayon::ThreadPool,
    pub gen_threads: Vec<(ChunkKey, Receiver<ChunkGenOutput>)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: Arc<dyn WorldGenerator<C>>,
    pub path: Option<PathBuf>,
}

impl<C: Chunk> Debug for ChunkHandler<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkHandler")
            .field("load_queue", &self.load_queue)
            .field("gen_pool", &self.gen_pool)
            .field("gen_threads", &self.gen_threads)
            .field("screen_size", &self.screen_size)
            .field("path", &self.path)
            .finish()
    }
}

#[allow(clippy::cast_lossless)]
pub type ChunkGenOutput = (
    ChunkKey,
    Box<[MaterialInstance; CHUNK_AREA]>,
    Box<[Color; CHUNK_AREA]>,
    Box<[MaterialInstance; CHUNK_AREA]>,
    Box<[Color; CHUNK_AREA]>,
);

#[derive(Serialize, Deserialize)]
struct ChunkSaveFormat {
    pixels: Vec<MaterialInstance>,
    colors: Vec<Color>,
}

pub struct ChunkTickContext<'a> {
    pub tick_time: u32,
    pub settings: &'a Settings,
    pub world: &'a mut specs::World,
    pub physics: &'a mut Physics,
    pub registries: &'a Arc<Registries>,
    pub seed: i32,
    pub file_helper: &'a FileHelper,
}

struct Zones {
    unload: Rect<i32>,
    load: Rect<i32>,
    active: Rect<i32>,
    #[allow(dead_code)]
    screen: Rect<i32>,
}

impl<C: Chunk + SidedChunk + Send + Sync + 'static> ChunkHandler<C>
where
    <<C as SidedChunk>::S as SidedChunkData>::TileEntityData: TileEntitySided<D = C>,
{
    const MAX_LOAD_PER_TICK: usize = 64;
    const MAX_SPAWN_GENERATE_PER_TICK: usize = 32;

    /// If num active + num cached chunks < `FAST_JOIN_THRESHOLD`, use `FAST_JOIN_PER_TICK` else `SLOW_JOIN_PER_TICK`
    const FAST_JOIN_THRESHOLD: usize = 4;
    const SLOW_JOIN_PER_TICK: usize = 8;
    const FAST_JOIN_PER_TICK: usize = 32;

    // #[profiling::function] // breaks clippy
    #[warn(clippy::too_many_arguments)] // TODO
    #[allow(clippy::needless_pass_by_value)]
    pub fn tick(&mut self, mut ctx: ChunkTickContext) {
        profiling::scope!("tick");

        let loader_zones = self.calc_zones(ctx.world);

        if ctx.settings.load_chunks {
            self.queue_chunk_loading(&loader_zones);
            self.load_chunks(&ctx);
        }

        // switch chunks between cached and active
        if ctx.tick_time % 2 == 0 {
            self.update_active(&mut ctx, &loader_zones);
        }

        if ctx.settings.load_chunks && ctx.tick_time % 2 == 0 {
            let num_active = self
                .manager
                .chunks_iter()
                .filter(|c| c.state() == ChunkState::Active)
                .count();
            let num_cached = self
                .manager
                .chunks_iter()
                .filter(|c| c.state() == ChunkState::Cached)
                .count();

            // generate new chunks
            self.generate_chunks(&ctx, &loader_zones, num_active, num_cached);

            // unloading NotGenerated or Generating chunks
            // populate chunks
            self.populate_chunks_and_check_unload_generating(&mut ctx, &loader_zones, num_active);
        }

        if ctx.settings.simulate_chunks {
            self.simulate_chunks(&mut ctx);
        }

        self.tick_tile_entities(&mut ctx);
    }

    fn calc_zones(&self, world: &specs::World) -> Vec<Zones> {
        let (loaders, positions) =
            world.system_data::<(ReadStorage<Loader>, ReadStorage<Position>)>();

        (&loaders, &positions)
            .join()
            .map(|(_, pos)| Zones {
                unload: self.get_unload_zone((pos.x, pos.y)),
                load: self.get_load_zone((pos.x, pos.y)),
                active: self.get_active_zone((pos.x, pos.y)),
                screen: self.get_screen_zone((pos.x, pos.y)),
            })
            .collect()
    }

    fn queue_chunk_loading(&mut self, loader_zones: &[Zones]) {
        profiling::scope!("queue_chunk_loading");
        for zones in loader_zones {
            for px in zones.load.range_lr().step_by(CHUNK_SIZE.into()) {
                for py in zones.load.range_tb().step_by(CHUNK_SIZE.into()) {
                    let chunk_pos = pixel_to_chunk_pos(px.into(), py.into());
                    self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
                }
            }
        }
    }

    fn load_chunks(&mut self, ctx: &ChunkTickContext) {
        profiling::scope!("chunk loading");

        let (loaders, positions) = ctx
            .world
            .system_data::<(ReadStorage<Loader>, ReadStorage<Position>)>();

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

        for _ in 0..Self::MAX_LOAD_PER_TICK {
            // TODO: don't load queued chunks if they are no longer in range
            if let Some(to_load) = self.load_queue.pop() {
                let c = self.load_chunk(to_load.0, to_load.1);
                if to_load == (0, 0) {
                    let ase = AsepriteFile::read_file(
                        &ctx.file_helper.asset_path("data/tile_entity/test/test.ase"),
                    )
                    .unwrap();
                    c.add_tile_entity(TileEntityCommon {
                        material_rect: MaterialRect::load_from_ase(&ase, (-40, -40)),
                    });
                }
            }
        }
    }

    fn update_active(&mut self, ctx: &mut ChunkTickContext, loader_zones: &[Zones]) {
        profiling::scope!("chunk update A");

        let keys = self.manager.keys();
        let mut keep_map = vec![true; keys.len()];
        for i in 0..keys.len() {
            let key = keys[i];

            let state = self.manager.chunk_at(key).unwrap().state(); // copy
            let rect = Rect::new_wh(
                self.manager.chunk_at(key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                self.manager.chunk_at(key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                CHUNK_SIZE,
                CHUNK_SIZE,
            );

            match state {
                ChunkState::Cached => {
                    if !loader_zones.iter().any(|z| rect.intersects(&z.unload)) {
                        if let Err(e) = self.save_chunk(key) {
                            log::error!("Chunk @ {}, {} failed to save: {:?}", key.0, key.1, e);
                        }
                        if let Err(e) = self.unload_chunk(key, ctx.physics) {
                            log::error!("Chunk @ {}, {} failed to unload: {:?}", key.0, key.1, e);
                        }
                        keep_map[i] = false;
                    } else if loader_zones.iter().any(|z| rect.intersects(&z.active)) {
                        let (chunk_x, chunk_y) = key;
                        if [
                            self.chunk_at((chunk_x - 1, chunk_y - 1)),
                            self.chunk_at((chunk_x, chunk_y - 1)),
                            self.chunk_at((chunk_x + 1, chunk_y - 1)),
                            self.chunk_at((chunk_x - 1, chunk_y)),
                            self.chunk_at((chunk_x, chunk_y)),
                            self.chunk_at((chunk_x + 1, chunk_y)),
                            self.chunk_at((chunk_x - 1, chunk_y + 1)),
                            self.chunk_at((chunk_x, chunk_y + 1)),
                            self.chunk_at((chunk_x + 1, chunk_y + 1)),
                        ]
                        .iter()
                        .all(|ch| {
                            if ch.is_none() {
                                return false;
                            }

                            let state = ch.unwrap().state();

                            matches!(state, ChunkState::Cached | ChunkState::Active)
                        }) {
                            self.manager
                                .chunk_at_mut(key)
                                .unwrap()
                                .set_state(ChunkState::Active);
                            self.manager
                                .chunk_at_mut(key)
                                .unwrap()
                                .set_dirty_rect(Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE)));
                        }
                    }
                },
                ChunkState::Active => {
                    if !loader_zones.iter().any(|z| rect.intersects(&z.active)) {
                        self.manager
                            .chunk_at_mut(key)
                            .unwrap()
                            .set_state(ChunkState::Cached);
                    }
                },
                _ => {},
            }
        }

        if ctx.settings.load_chunks {
            let mut iter = keep_map.iter();
            unsafe { self.manager.raw_mut() }.retain(|_, _| *iter.next().unwrap());
        }
    }

    fn generate_chunks(
        &mut self,
        ctx: &ChunkTickContext,
        loader_zones: &[Zones],
        num_active: usize,
        num_cached: usize,
    ) {
        profiling::scope!("generate_chunks");

        let (loaders, positions) = ctx
            .world
            .system_data::<(ReadStorage<Loader>, ReadStorage<Position>)>();

        // get keys for all chunks sorted by distance to nearest loader
        let mut keys = unsafe { self.manager.raw_mut().iter() }
            .filter_map(|(k, c)| {
                if c.state() == ChunkState::NotGenerated {
                    Some(k)
                } else {
                    None
                }
            })
            .copied()
            .collect::<Vec<ChunkKey>>();
        if !loaders.is_empty() {
            profiling::scope!("sort");
            keys.sort_by(|a, b| {
                let a = self.manager.chunk_at(*a).unwrap();
                let b = self.manager.chunk_at(*b).unwrap();
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
                self.manager.chunk_at(*key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                self.manager.chunk_at(*key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                CHUNK_SIZE,
                CHUNK_SIZE,
            );

            // keys are filtered by state == NotGenerated already
            assert!(self.manager.chunk_at(*key).unwrap().state() == ChunkState::NotGenerated);

            // start generating chunks waiting to generate
            if loader_zones.iter().any(|z| rect.intersects(&z.unload)) && num_loaded_this_tick < Self::MAX_SPAWN_GENERATE_PER_TICK {
                let chunk_x = self.manager.chunk_at_mut(*key).unwrap().chunk_x();
                let chunk_y = self.manager.chunk_at_mut(*key).unwrap().chunk_y();

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
                                        == (CHUNK_AREA)
                                    {
                                        let chunk =  self.manager.chunk_at_mut(*key).unwrap();
                                        chunk.set_state(ChunkState::Cached);
                                        chunk.set_pixels(save.pixels.try_into().unwrap());
                                        chunk.mark_dirty();
                                        let _: Result<(), _> = chunk.generate_mesh();

                                        if save.colors.len()
                                            == (CHUNK_SIZE as usize
                                                * CHUNK_SIZE as usize
                                                * 4)
                                        {
                                            chunk.set_pixel_colors(save.colors.try_into().unwrap());
                                        } else {
                                            log::error!("colors Vec is the wrong size: {} (expected {})", save.colors.len(), CHUNK_AREA * 4);
                                            chunk.refresh();
                                        }

                                        should_generate = false;
                                    } else {
                                        log::error!("pixels Vec is the wrong size: {} (expected {})", save.pixels.len(), CHUNK_AREA);
                                        self.manager
                                            .chunk_at_mut(*key)
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
                                    self.manager
                                        .chunk_at_mut(*key)
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
                            self.manager
                                .chunk_at_mut(*key)
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
                self.spawn_chunk_generation(ctx, key, chunk_x, chunk_y);
            }
        }

        // get data from a number of finished generation tasks
        // if less than `FAST_JOIN_THRESHOLD` chunks are finished, join a lot more
        // this means that at the start of loading a world chunks appear a lot faster, at the cost of added lag
        let mut generated = vec![];
        self.gen_threads.retain_mut(|(_, v)| {
            if generated.len()
                < if num_cached + num_active < Self::FAST_JOIN_THRESHOLD {
                    Self::FAST_JOIN_PER_TICK
                } else {
                    Self::SLOW_JOIN_PER_TICK
                }
            {
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

                self.manager.chunk_at_mut(key).map(|chunk| {
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
            unsafe { self.manager.raw_mut().get_many_var_mut(&keys) }
                .unwrap()
                .into_par_iter()
                .for_each(|chunk| {
                    profiling::scope!("populate thread");
                    pops.populate(0, &mut [&mut chunk.data], ctx.seed, ctx.registries);
                });
        }
    }

    fn spawn_chunk_generation(
        &mut self,
        ctx: &ChunkTickContext,
        key: ChunkKey,
        chunk_x: i32,
        chunk_y: i32,
    ) {
        // need to clone since these need to be 'static
        let generator = self.generator.clone();
        let reg = ctx.registries.clone();
        let (tx, rx) = futures::channel::oneshot::channel();
        let seed = ctx.seed;
        self.gen_pool.spawn_fifo(move || {
            profiling::register_thread!("Generation thread");
            profiling::scope!("chunk");

            // these arrays are too large for the stack

            let mut pixels = Box::new([(); CHUNK_AREA].map(|_| MaterialInstance::air()));

            #[allow(clippy::cast_lossless)]
            let mut colors = Box::new([Color::TRANSPARENT; CHUNK_AREA]);

            let mut background = Box::new([(); CHUNK_AREA].map(|_| MaterialInstance::air()));

            #[allow(clippy::cast_lossless)]
            let mut background_colors = Box::new([Color::TRANSPARENT; CHUNK_AREA]);

            generator.generate(
                (chunk_x, chunk_y),
                GenBuffers::new(
                    &mut pixels,
                    &mut colors,
                    &mut background,
                    &mut background_colors,
                ),
                GenContext { seed, registries: &reg },
            );

            tx.send((key, pixels, colors, background, background_colors))
                .unwrap();
        });

        self.gen_threads.push((key, rx));
    }

    // TODO: split this (figure out why were these two tasks combined originally)
    fn populate_chunks_and_check_unload_generating(
        &mut self,
        ctx: &mut ChunkTickContext,
        loader_zones: &[Zones],
        num_active: usize,
    ) {
        profiling::scope!("populate_chunks_and_check_unload_generating");

        let keys = self.manager.keys();
        let mut keep_map = vec![true; keys.len()];
        let mut populated_num = 0;
        for i in 0..keys.len() {
            profiling::scope!("chunk");
            let key = keys[i];
            let state = self.manager.chunk_at(key).unwrap().state(); // copy
            let rect = Rect::new_wh(
                self.manager.chunk_at(key).unwrap().chunk_x() * i32::from(CHUNK_SIZE),
                self.manager.chunk_at(key).unwrap().chunk_y() * i32::from(CHUNK_SIZE),
                CHUNK_SIZE,
                CHUNK_SIZE,
            );

            match state {
                ChunkState::NotGenerated => {
                    profiling::scope!("NotGenerated");
                    if !loader_zones.iter().any(|z| rect.intersects(&z.unload)) {
                        if let Err(e) = self.save_chunk(key) {
                            log::error!("Chunk @ {}, {} failed to save: {:?}", key.0, key.1, e);
                        };
                        if let Err(e) = self.unload_chunk(key, ctx.physics) {
                            log::error!("Chunk @ {}, {} failed to unload: {:?}", key.0, key.1, e);
                        };
                        keep_map[i] = false;
                    }
                },
                ChunkState::Generating(cur_stage) => {
                    profiling::scope!("Generating");
                    let chunk_x = self.manager.chunk_at(key).unwrap().chunk_x();
                    let chunk_y = self.manager.chunk_at(key).unwrap().chunk_y();

                    let max_stage = self.generator.max_gen_stage();

                    if cur_stage >= max_stage {
                        profiling::scope!("finish");
                        let _: Result<(), _> =
                            self.manager.chunk_at_mut(key).unwrap().generate_mesh();

                        self.manager
                            .chunk_at_mut(key)
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
                            && {
                                profiling::scope!("check neighbors");
                                [
                                    self.chunk_at((chunk_x - 1, chunk_y - 1)),
                                    self.chunk_at((chunk_x, chunk_y - 1)),
                                    self.chunk_at((chunk_x + 1, chunk_y - 1)),
                                    self.chunk_at((chunk_x - 1, chunk_y)),
                                    self.chunk_at((chunk_x, chunk_y)),
                                    self.chunk_at((chunk_x + 1, chunk_y)),
                                    self.chunk_at((chunk_x - 1, chunk_y + 1)),
                                    self.chunk_at((chunk_x, chunk_y + 1)),
                                    self.chunk_at((chunk_x + 1, chunk_y + 1)),
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
                            }
                        {
                            profiling::scope!("check populate");

                            let range = i32::from(cur_stage + 1);
                            let mut keys = Vec::with_capacity(((range * 2) * (range * 2)) as usize);

                            // try to gather the nearby chunks needed to populate this one
                            for y in -range..=range {
                                for x in -range..=range {
                                    keys.push((chunk_x + x, chunk_y + y));
                                }
                            }

                            let chunks = unsafe { self.manager.raw_mut().get_many_var_mut(&keys) };

                            // if we failed to get all nearby chunks, don't populate and don't go to the next stage
                            if let Some((true, chunks)) =
                                chunks.map(|chs| (chs.iter().all(|c| c.pixels().is_some()), chs))
                            {
                                profiling::scope!("populating");
                                let mut chunks_data: Vec<_> =
                                    chunks.into_iter().map(|c| &mut c.data).collect();

                                if cur_stage + 1 == 1 {
                                    let mut chunk_ctx =
                                        ChunkContext::<1, C>::new(&mut chunks_data).unwrap();
                                    let mut rng = StdRng::seed_from_u64(
                                        ctx.seed as u64
                                            + u64::from(chunk_index(
                                                chunk_ctx.center_chunk().0,
                                                chunk_ctx.center_chunk().1,
                                            )),
                                    );
                                    for feat in self.generator.features() {
                                        feat.generate(
                                            &mut chunk_ctx,
                                            ctx.seed,
                                            &mut rng,
                                            ctx.registries,
                                            ctx.world,
                                        );
                                        ctx.world.maintain();
                                    }
                                }

                                self.generator.populators().populate(
                                    cur_stage + 1,
                                    &mut chunks_data,
                                    ctx.seed,
                                    ctx.registries,
                                );

                                self.manager
                                    .chunk_at_mut(key)
                                    .unwrap()
                                    .set_state(ChunkState::Generating(cur_stage + 1));

                                populated_num += 1;
                            }
                        }

                        {
                            profiling::scope!("check unload");
                            if !loader_zones.iter().any(|z| rect.intersects(&z.unload)) {
                                if let Err(e) = self.save_chunk(key) {
                                    log::error!(
                                        "Chunk @ {}, {} failed to save: {:?}",
                                        key.0,
                                        key.1,
                                        e
                                    );
                                };
                                if let Err(e) = self.unload_chunk(key, ctx.physics) {
                                    log::error!(
                                        "Chunk @ {}, {} failed to unload: {:?}",
                                        key.0,
                                        key.1,
                                        e
                                    );
                                };
                                keep_map[i] = false;
                            }
                        }
                    }
                },
                _ => {},
            }
        }

        // tick structures
        let mut update_structures = UpdateStructureNodes {
            chunk_handler: self,
            registries: ctx.registries.clone(),
        };
        update_structures.run_now(ctx.world);
        ctx.world.maintain();

        let mut iter = keep_map.iter();
        unsafe { self.manager.raw_mut() }.retain(|_, _| *iter.next().unwrap());
    }

    fn simulate_chunks(&mut self, ctx: &mut ChunkTickContext) {
        profiling::scope!("simulate_chunks");

        let mut old_dirty_rects = ahash::AHashMap::with_capacity(128);
        let mut keys_for_phases = [
            Vec::with_capacity(32),
            Vec::with_capacity(32),
            Vec::with_capacity(32),
            Vec::with_capacity(32),
        ];

        {
            profiling::scope!("pre prep");
            for (key, ch) in unsafe { self.manager.raw_mut().iter_mut() } {
                let rect = ch.dirty_rect();
                ch.set_dirty_rect(None);
                old_dirty_rects.insert(*key, rect);
                if ch.state() == ChunkState::Active {
                    keys_for_phases[chunk_update_order(key.0, key.1) as usize].push(*key);
                }
            }
        }

        #[allow(unused_variables)] // false positive
        for (tick_phase, keys) in keys_for_phases.into_iter().enumerate() {
            profiling::scope!("phase", format!("phase {tick_phase}").as_str());
            let mut to_exec = Vec::with_capacity(keys.len());
            {
                profiling::scope!("prep");
                for key in keys {
                    let ch_pos = key;
                    profiling::scope!("iter");

                    // SAFETY: the same chunks' arrays may be modified mutably on multiple threads at once, which is necessary for multithreading
                    // However, ticking a chunk can only affect pixels within CHUNK_SIZE/2 of the center chunk (this is unchecked)
                    //   and the 4-phase thing ensures no chunks directly next to each other are ticked at the same time
                    //   so multiple threads will not modify the same index in the arrays at the same time
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
                        .map(|(x, y)| {
                            let chunk = self.manager.chunk_at_mut((ch_pos.0 + x, ch_pos.1 + y));
                            chunk.and_then(|c| {
                                c.pixels_mut().as_mut().map(|raw| {
                                    // blatantly bypassing the borrow checker, see safety comment above
                                    unsafe { &*(raw.as_mut() as *mut _ as *const _) }
                                }).map(|pixels| {
                                    // blatantly bypassing the borrow checker, see safety comment above
                                    // TODO: I'm not sure if doing this while the data is already in a `&[UnsafeCell<_>; _]` is UB

                                    let raw: *mut [Color; CHUNK_AREA] = c.colors_mut();
                                    let colors = unsafe { &*(raw as *const [UnsafeCell<Color>; CHUNK_AREA]) };

                                    let raw: *mut [[f32; 4]; CHUNK_AREA] = c.lights_mut();
                                    let lights = unsafe { &*(raw as *const [UnsafeCell<[f32; 4]>; CHUNK_AREA]) };

                                    let dirty_rect = *old_dirty_rects
                                        .get(&(ch_pos.0 + x, ch_pos.1 + y))
                                        .unwrap();

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
                        .into_iter()
                        .collect::<Option<Vec<_>>>()
                        .map(|v| v.try_into().unwrap())
                    else {
                        continue;
                    };

                    to_exec.push((ch_pos, arr));
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
                    let reg = ctx.registries.clone();
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
                        ctx.world
                            .write_resource::<ParticleSystem>()
                            .active
                            .append(&mut parts);
                    }

                    for i in 0..9 {
                        let rel_ch_x = (i % 3) - 1;
                        let rel_ch_y = (i / 3) - 1;

                        let ch = self
                            .manager
                            .chunk_at_mut((ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y))
                            .unwrap();

                        // TODO: clean up this dirty rect code

                        if dirty_info[i as usize].0 {
                            ch.mark_dirty();
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

                            let mut r = ch.dirty_rect();
                            match r {
                                Some(current) => {
                                    r = Some(current.union(neighbor_rect));
                                },
                                None => {
                                    r = Some(neighbor_rect);
                                },
                            }
                            ch.set_dirty_rect(r);
                        }

                        if let Some(new) = dirty_info[i as usize].1 {
                            let mut r = ch.dirty_rect();
                            match r {
                                Some(current) => {
                                    r = Some(current.union(new));
                                },
                                None => {
                                    r = Some(new);
                                },
                            }
                            ch.set_dirty_rect(r);
                        }
                    }
                }
            }
        }
    }

    fn tick_tile_entities(&mut self, ctx: &mut ChunkTickContext) {
        profiling::scope!("tick_tile_entities");
        self.manager.query_each(|mut q| {
            q.for_each_with(
                |ch| ch.sided_tile_entities_removable(),
                |te, chunks| {
                    te.tick(TileEntityTickContext::<C> {
                        tick_time: ctx.tick_time,
                        registries: ctx.registries,
                        file_helper: ctx.file_helper,
                        chunks,
                    });
                },
            );
        });
    }
}

impl<C: Chunk> ChunkQuery for ChunkHandler<C> {
    type D = C;

    #[inline]
    fn chunk_at(&self, chunk_pos: ChunkKey) -> Option<&chunksystem::Chunk<Self::D>> {
        self.manager.chunk_at(chunk_pos)
    }

    #[inline]
    fn chunk_at_mut(&mut self, chunk_pos: ChunkKey) -> Option<&mut chunksystem::Chunk<Self::D>> {
        self.manager.chunk_at_mut(chunk_pos)
    }

    #[inline]
    fn chunks_iter(&self) -> chunksystem::BoxedIterator<&chunksystem::Chunk<Self::D>> {
        self.manager.chunks_iter()
    }

    #[inline]
    fn chunks_iter_mut(&mut self) -> chunksystem::BoxedIterator<&mut chunksystem::Chunk<Self::D>> {
        self.manager.chunks_iter_mut()
    }

    #[inline]
    fn kv_iter(&self) -> chunksystem::BoxedIterator<(ChunkKey, &chunksystem::Chunk<Self::D>)> {
        self.manager.kv_iter()
    }

    #[inline]
    fn kv_iter_mut(
        &mut self,
    ) -> chunksystem::BoxedIterator<(ChunkKey, &mut chunksystem::Chunk<Self::D>)> {
        self.manager.kv_iter_mut()
    }

    #[inline]
    fn keys(&self) -> Vec<ChunkKey> {
        self.manager.keys()
    }

    #[inline]
    fn query_one(&mut self, chunk_pos: ChunkKey) -> Option<chunksystem::ChunkQueryOne<Self::D>> {
        self.manager.query_one(chunk_pos)
    }

    #[inline]
    fn is_chunk_loaded(&self, chunk_pos: (i32, i32)) -> bool {
        self.manager.is_chunk_loaded(chunk_pos)
    }
}

impl<C: Chunk> ChunkHandler<C> {
    // #[profiling::function]
    pub fn new(generator: impl WorldGenerator<C> + 'static, path: Option<PathBuf>) -> Self {
        ChunkHandler {
            manager: ChunkManager::new_with_capacity(1000),
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

    #[profiling::function]
    pub fn save_chunk(&mut self, index: ChunkKey) -> Result<(), Box<dyn std::error::Error>> {
        let chunk = self.manager.chunk_at_mut(index).ok_or("Chunk not loaded")?;
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

    pub fn unload_all_chunks(
        &mut self,
        physics: &mut Physics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[allow(clippy::for_kv_map)] // want ? to work
        let keys = self.manager.keys();
        for i in keys {
            self.unload_chunk(i, physics)?;
        }
        self.manager.clear();
        Ok(())
    }

    pub fn save_all_chunks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[allow(clippy::for_kv_map)] // want ? to work
        let keys = self.manager.keys();
        for i in keys {
            self.save_chunk(i)?;
        }
        Ok(())
    }

    #[profiling::function]
    pub fn queue_load_chunk(&mut self, chunk_x: i32, chunk_y: i32) -> bool {
        // make sure not loaded
        if self.is_chunk_loaded((chunk_x, chunk_y)) {
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

    pub fn force_update_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(ch) = self.manager.chunk_at_mut((chunk_x, chunk_y)) {
            ch.set_dirty_rect(Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE)));
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    #[profiling::function]
    fn unload_chunk(
        &mut self,
        index: ChunkKey,
        physics: &mut Physics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chunk = self.manager.chunk_at_mut(index).unwrap();
        if let Some(ChunkRigidBodyState::Active(handle)) = chunk.rigidbody() {
            physics.remove_rigidbody(*handle);
            chunk.set_rigidbody(None);
        }

        Ok(())
    }

    #[profiling::function]
    fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32) -> &mut C {
        let chunk = Chunk::new_empty(chunk_x, chunk_y);
        let i = (chunk_x, chunk_y);
        self.manager.insert(i, chunk);
        self.manager.chunk_at_mut(i).unwrap()
    }

    #[profiling::function]
    #[inline]
    pub fn get_zone(&self, center: (f64, f64), padding: u16) -> Rect<i32> {
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
    #[inline]
    pub fn get_screen_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, 0)
    }

    #[profiling::function]
    #[inline]
    pub fn get_active_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE)
    }

    #[profiling::function]
    #[inline]
    pub fn get_load_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE * 10)
    }

    #[profiling::function]
    #[inline]
    pub fn get_unload_zone(&self, center: (f64, f64)) -> Rect<i32> {
        self.get_zone(center, CHUNK_SIZE * 15)
    }
}
