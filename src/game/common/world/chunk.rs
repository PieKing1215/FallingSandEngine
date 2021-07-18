
use crate::game::client::world::ChunkGraphics;
use crate::game::{common::world::simulator::Simulator};
use crate::game::common::Settings;
use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use lazy_static::lazy_static;
use sdl2::rect::Rect;
use tokio::runtime::Runtime;

use super::gen::WorldGenerator;
use crate::game::common::world::material::MaterialInstance;

pub const CHUNK_SIZE: u16 = 128;

pub struct Chunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub graphics: Box<ChunkGraphics>,
    pub dirty_rect: Option<Rect>,
}

impl<'ch> Chunk {
    pub fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            graphics: Box::new(ChunkGraphics {
                texture: None,
                pixel_data: [0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)],
                dirty: true,
                was_dirty: true,
            }),
            dirty_rect: None,
        }
    }

    pub fn refresh(&mut self){
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                self.graphics.set(x, y, self.pixels.unwrap()[(x + y * CHUNK_SIZE) as usize].color).unwrap();
            }
        }
    }

    // #[profiling::function]
    pub fn update_graphics(&mut self) -> Result<(), String> {

        self.graphics.update_texture().map_err(|e| "ChunkGraphics::update_texture failed.")?;

        Ok(())
    }

    // #[profiling::function] // huge performance impact
    pub fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {

            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                px[i] = mat;
                self.graphics.set(x, y, px[i].color)?;

                self.dirty_rect = Some(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    pub fn apply_diff(&mut self, diff: &Vec<(u16, u16, MaterialInstance)>) {
        diff.iter().for_each(|(x, y, mat)| {
            self.set(*x, *y, *mat).unwrap(); // TODO: handle this Err
        });
    }
}

#[derive(Clone, Copy)]
pub enum ChunkState {
    NotGenerated,
    Generating(u8), // stage
    Cached,
    Active,
}

pub struct ChunkHandler<T: WorldGenerator + Copy + Send + Sync + 'static> {
    pub loaded_chunks: HashMap<u32, Box<Chunk>>,
    load_queue: Vec<(i32, i32)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: T,
}

impl<'a, T: WorldGenerator + Copy + Send + Sync + 'static> ChunkHandler<T> {
    #[profiling::function]
    pub fn new(generator: T) -> Self {
        ChunkHandler {
            loaded_chunks: HashMap::new(),
            load_queue: vec![],
            screen_size: (1920 / 2, 1080 / 2),
            generator
        }
    }

    #[profiling::function]
    pub fn update_chunk_graphics(&mut self){
        let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
        for i in 0..keys.len() {
            let key = keys[i];
            self.loaded_chunks.get_mut(&key).unwrap().graphics.was_dirty = self.loaded_chunks.get_mut(&key).unwrap().graphics.dirty;
            self.loaded_chunks.get_mut(&key).unwrap().update_graphics().unwrap();
        }
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, loaders: Vec<(f64, f64)>, settings: &Settings){ // TODO: `camera` should be replaced with like a vec of entities or something
        
        let unload_zone: Vec<Rect> = loaders.iter().map(|l| self.get_unload_zone(*l)).collect();
        let load_zone: Vec<Rect> = loaders.iter().map(|l| self.get_load_zone(*l)).collect();
        let active_zone: Vec<Rect> = loaders.iter().map(|l| self.get_active_zone(*l)).collect();
        let _screen_zone: Vec<Rect> = loaders.iter().map(|l| self.get_screen_zone(*l)).collect();
        
        if settings.load_chunks {
            {
                profiling::scope!("queue chunk loading");
                load_zone.iter().for_each(|load_zone| {
                    for px in (load_zone.x .. load_zone.x + load_zone.w).step_by(CHUNK_SIZE.into()) {
                        for py in (load_zone.y .. load_zone.y + load_zone.h).step_by(CHUNK_SIZE.into()) {
                            let chunk_pos = self.pixel_to_chunk_pos(px.into(), py.into());
                            self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
                        }
                    }
                });
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
            let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
            for i in 0..keys.len() {
                let key = keys[i];
                
                let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                let rect = Rect::new(self.loaded_chunks.get(&key).unwrap().chunk_x * CHUNK_SIZE as i32, self.loaded_chunks.get(&key).unwrap().chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

                match state {
                    ChunkState::Cached => {
                        if !unload_zone.iter().any(|z| rect.has_intersection(*z)) {
                            self.unload_chunk(&self.loaded_chunks.get(&key).unwrap());
                            keep_map[i] = false;
                        }else if active_zone.iter().any(|z| rect.has_intersection(*z)) {
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().chunk_x;
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().chunk_y;
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
                            ].iter().all(|ch| {
                                if ch.is_none() {
                                    return false;
                                }

                                let state = ch.unwrap().state;

                                match state {
                                    ChunkState::Cached | ChunkState::Active => true,
                                    _ => false,
                                }
                            }) {
                                self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Active;
                                self.loaded_chunks.get_mut(&key).unwrap().dirty_rect = Some(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));
                            }
                        }
                    },
                    ChunkState::Active => {
                        if !active_zone.iter().any(|z| rect.has_intersection(*z)) {
                            self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Cached;
                        }
                    }
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

                let mut keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
                keys.sort_by(|a, b| {
                    let c1_x = self.loaded_chunks.get(a).unwrap().chunk_x * CHUNK_SIZE as i32;
                    let c1_y = self.loaded_chunks.get(a).unwrap().chunk_y * CHUNK_SIZE as i32;
                    let c2_x = self.loaded_chunks.get(b).unwrap().chunk_x * CHUNK_SIZE as i32;
                    let c2_y = self.loaded_chunks.get(b).unwrap().chunk_y * CHUNK_SIZE as i32;

                    let d1 = loaders.iter().map(|l| {
                        let x = (l.0 as i32 - c1_x).abs();
                        let y = (l.1 as i32 - c1_y).abs();
                        x + y
                    }).min().unwrap();

                    let d2 = loaders.iter().map(|l| {
                        let x = (l.0 as i32 - c2_x).abs();
                        let y = (l.1 as i32 - c2_y).abs();
                        x + y
                    }).min().unwrap();

                    d1.cmp(&d2)
                });
                let mut to_exec = vec![];
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                    let rect = Rect::new(self.loaded_chunks.get(&key).unwrap().chunk_x * CHUNK_SIZE as i32, self.loaded_chunks.get(&key).unwrap().chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

                    match state {
                        ChunkState::NotGenerated => {
                            if !unload_zone.iter().any(|z| rect.has_intersection(*z)) {

                            }else if num_loaded_this_tick < 32 {
                                // TODO: load from file
                                
                                self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Generating(0);
                                
                                let chunk_x = self.loaded_chunks.get_mut(&key).unwrap().chunk_x;
                                let chunk_y = self.loaded_chunks.get_mut(&key).unwrap().chunk_y;

                                to_exec.push((i, chunk_x, chunk_y));
                                // generation_pool.spawn_ok(fut);
                                num_loaded_this_tick += 1;
                            }
                        },
                        _ => {},
                    }
                }

                lazy_static! {
                    static ref RT: Runtime = Runtime::new().unwrap();
                }

                if to_exec.len() > 0 {
                    // println!("a {}", to_exec.len());

                    let gen = self.generator;
                    // WARNING: LEAK
                    let futs: Vec<_> = Box::leak(Box::new(to_exec)).iter().map(|e| Arc::from(e)).map(|e| async move {
                        let mut pixels = Box::new([MaterialInstance::air(); (CHUNK_SIZE * CHUNK_SIZE) as usize]);
                        let mut colors = Box::new([0; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]);
                        gen.generate(e.1, e.2, 2, &mut pixels, &mut colors); // TODO: non constant seed
                        // println!("{}", e.0);
                        (e.0, pixels, colors)
                    }).collect();
                    let futs2: Vec<_> = futs.into_iter().map(|f| RT.spawn(f)).collect();
                    let b = RT.block_on(join_all(futs2));
                    for i in 0..b.len() {
                        let p = b[i].as_ref().unwrap();
                        // println!("{} {}", i, p.0);
                        self.loaded_chunks.get_mut(&keys[p.0]).unwrap().pixels = Some(*p.1);
                        self.loaded_chunks.get_mut(&keys[p.0]).unwrap().graphics.replace(*p.2);
                    }
                }

            }

            // unloading NotGenerated or Generating chunks
            // populate chunks
            if tick_time % 2 == 0 {
                profiling::scope!("chunk update C");

                let mut keep_map = vec![true; self.loaded_chunks.len()];
                let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                    let rect = Rect::new(self.loaded_chunks.get(&key).unwrap().chunk_x * CHUNK_SIZE as i32, self.loaded_chunks.get(&key).unwrap().chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

                    match state {
                        ChunkState::NotGenerated => {
                            if !unload_zone.iter().any(|z| rect.has_intersection(*z)) {
                                self.unload_chunk(&self.loaded_chunks.get(&key).unwrap());
                                keep_map[i] = false;
                            }
                        },
                        ChunkState::Generating(stage) => {
                            let chunk_x = self.loaded_chunks.get(&key).unwrap().chunk_x;
                            let chunk_y = self.loaded_chunks.get(&key).unwrap().chunk_y;

                            let max_stage = self.generator.max_gen_stage();

                            if stage >= max_stage {
                                self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Cached;
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
                                ].iter().all(|ch| {
                                    if ch.is_none() {
                                        return false;
                                    }

                                    let state = ch.unwrap().state;

                                    match state {
                                        ChunkState::Cached | ChunkState::Active => true,
                                        ChunkState::Generating(st) if st >= stage => true,
                                        _ => false,
                                    }
                                }) {
                                    self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Generating(stage + 1);
                                }

                                if !unload_zone.iter().any(|z| rect.has_intersection(*z)) {
                                    self.unload_chunk(&self.loaded_chunks.get(&key).unwrap());
                                    keep_map[i] = false;
                                }
                            }
                        }
                        _ => {},
                    }
                }

                let mut iter = keep_map.iter();
                self.loaded_chunks.retain(|_, _| *iter.next().unwrap());
            }
        }

        if tick_time % 1 == 0 {
            profiling::scope!("chunk simulate");

            lazy_static! {
                static ref RT: Runtime = Runtime::new().unwrap();
            }

            let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
            let mut old_dirty_rects: HashMap<u32, Option<Rect>> = HashMap::with_capacity(keys.len());

            for i in 0..keys.len() {
                let key = keys[i];
                old_dirty_rects.insert(key, self.loaded_chunks.get(&key).unwrap().dirty_rect.clone());
                self.loaded_chunks.get_mut(&key).unwrap().dirty_rect = None;
            }

            for tick_phase in 0..4 {
                profiling::scope!("phase", format!("phase {}", tick_phase).as_str());
                let mut to_exec = vec![];
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                    let ch_pos = (self.loaded_chunks.get(&key).unwrap().chunk_x, self.loaded_chunks.get(&key).unwrap().chunk_y);
                    if self.chunk_update_order(ch_pos.0, ch_pos.1) == tick_phase {
                        match state {
                            ChunkState::Active => {
                                profiling::scope!("iter");

                                if old_dirty_rects.get(&key).is_some() {
                                    let ch00: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap().pixels.as_mut().unwrap();
                                    let ch10: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap().pixels.as_mut().unwrap();
                                    let ch20: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap().pixels.as_mut().unwrap();
                                    let ch01: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap().pixels.as_mut().unwrap();
                                    let ch11: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap().pixels.as_mut().unwrap();
                                    let ch21: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap().pixels.as_mut().unwrap();
                                    let ch02: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap().pixels.as_mut().unwrap();
                                    let ch12: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap().pixels.as_mut().unwrap();
                                    let ch22: *mut [MaterialInstance; (CHUNK_SIZE as usize * CHUNK_SIZE as usize)] = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap().pixels.as_mut().unwrap();
                                    let arr = [
                                        ch00 as usize, ch10 as usize, ch20 as usize, 
                                        ch01 as usize, ch11 as usize, ch21 as usize, 
                                        ch02 as usize, ch12 as usize, ch22 as usize ];

                                    let gr_ch00: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap().graphics.pixel_data;
                                    let gr_ch10: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap().graphics.pixel_data;
                                    let gr_ch20: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap().graphics.pixel_data;
                                    let gr_ch01: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap().graphics.pixel_data;
                                    let gr_ch11: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap().graphics.pixel_data;
                                    let gr_ch21: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap().graphics.pixel_data;
                                    let gr_ch02: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap().graphics.pixel_data;
                                    let gr_ch12: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap().graphics.pixel_data;
                                    let gr_ch22: *mut [u8; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)] = &mut self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap().graphics.pixel_data;
                                    let gr_arr = [
                                        gr_ch00 as usize, gr_ch10 as usize, gr_ch20 as usize, 
                                        gr_ch01 as usize, gr_ch11 as usize, gr_ch21 as usize, 
                                        gr_ch02 as usize, gr_ch12 as usize, gr_ch22 as usize ];

                                    let dirty_ch00 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap();
                                    let dirty_ch10 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap();
                                    let dirty_ch20 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap();
                                    let dirty_ch01 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap();
                                    let dirty_ch11 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap();
                                    let dirty_ch21 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap();
                                    let dirty_ch02 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap();
                                    let dirty_ch12 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap();
                                    let dirty_ch22 = *old_dirty_rects.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap();
                                    let dirty_arr = [
                                        dirty_ch00, dirty_ch10, dirty_ch20, 
                                        dirty_ch01, dirty_ch11, dirty_ch21, 
                                        dirty_ch02, dirty_ch12, dirty_ch22 ];

                                    // let diff = self.simulate_chunk(arr);

                                    // for i in 0..9 {
                                    //     if diff[i].len() > 0 {
                                    //         let rel_ch_x = (i % 3) as i32 - 1;
                                    //         let rel_ch_y = (i / 3) as i32 - 1;
                                    //         self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap()
                                    //             .apply_diff(&diff[i]);
                                    //     }
                                    // }

                                    to_exec.push((i, ch_pos, arr, gr_arr, dirty_arr));
                                }
                            },
                            _ => {},
                        }
                    }
                }

                if to_exec.len() > 0 {
                    profiling::scope!("run simulation");
                    let futs: Vec<_> = Box::leak(Box::new(to_exec)).iter().map(|e| Arc::from(e)).map(|e: Arc<&(usize, (i32, i32), [usize; 9], [usize; 9], [Option<Rect>; 9])>| async move {
                        profiling::register_thread!("Simulation thread");
                        profiling::scope!("chunk");
                        let ch_pos = e.1;

                        let mut dirty = [false; 9];
                        let mut dirty_rects = e.4;
                        Simulator::simulate_chunk(ch_pos.0, ch_pos.1, e.2, e.3, &mut dirty, &mut dirty_rects);

                        (ch_pos, dirty, dirty_rects)
                    }).collect();
                    let futs2: Vec<_> = futs.into_iter().map(|f| RT.spawn(f)).collect();
                    let b: Vec<Result<((i32, i32), [bool; 9], [Option<Rect>; 9]), _>>;
                    {
                        profiling::scope!("wait for threads", format!("#futs = {}", futs2.len()).as_str());
                        b = RT.block_on(join_all(futs2));
                    }
                    for i in 0..b.len() {
                        profiling::scope!("apply");
                        let (ch_pos, dirty, dirty_rects) = b[i].as_ref().unwrap();
                        for i in 0..9 {
                            let rel_ch_x = (i % 3) as i32 - 1;
                            let rel_ch_y = (i / 3) as i32 - 1;

                            if dirty[i] {
                                self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap().graphics.dirty = true;
                            }

                            if i != 4 {
                                if let Some(_) = dirty_rects[4] {
                                    // let neighbor_rect = Rect::new(
                                    //     if rel_ch_x == -1 { (CHUNK_SIZE / 2).into() } else { 0 },
                                    //     if rel_ch_y == -1 { (CHUNK_SIZE / 2).into() } else { 0 },
                                    //     if rel_ch_x == 0 { (CHUNK_SIZE).into() } else { (CHUNK_SIZE / 2).into() },
                                    //     if rel_ch_y == 0 { (CHUNK_SIZE).into() } else { (CHUNK_SIZE / 2).into() }
                                    // );
                                    let neighbor_rect = Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
                                    let mut r = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap().dirty_rect;
                                    match r {
                                        Some(current) => {
                                            r = Some(current.union(neighbor_rect));
                                        },
                                        None => {
                                            r = Some(neighbor_rect);
                                        },
                                    }
                                    self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap().dirty_rect = r;
                                }
                            }
                            
                            if let Some(new) = dirty_rects[i] {
                                let mut r = self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap().dirty_rect;
                                match r {
                                    Some(current) => {
                                        r = Some(current.union(new));
                                    },
                                    None => {
                                        r = Some(new);
                                    },
                                }
                                self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap().dirty_rect = r;
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
        //                     let ch00 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch10 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch20 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch01 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch11 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch21 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
        //                     let ch02 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch12 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let ch22 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
        //                     let arr = [
        //                         ch00, ch10, ch20, 
        //                         ch01, ch11, ch21, 
        //                         ch02, ch12, ch22 ];

        //                     let diff = self.simulate_chunk(arr);

        //                     for i in 0..9 {
        //                         let rel_ch_x = (i % 3) as i32 - 1;
        //                         let rel_ch_y = (i / 3) as i32 - 1;
        //                         self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap()
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
    fn unload_chunk(&self, _chunk: &Chunk){
        // write to file, free textures, etc
    }

    #[profiling::function]
    pub fn queue_load_chunk(&mut self, chunk_x: i32, chunk_y: i32) -> bool {
        // make sure not loaded
        if self.is_chunk_loaded(chunk_x, chunk_y) {
            return false;
        }

        // make sure not loading
        if self.load_queue.iter().any(|ch| ch.0 == chunk_x && ch.1 == chunk_y) {
            return false;
        }

        self.load_queue.push((chunk_x, chunk_y));

        return true;
    }

    #[profiling::function]
    fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32){
        let chunk = Chunk::new_empty(chunk_x, chunk_y);
        self.loaded_chunks.insert(self.chunk_index(chunk_x, chunk_y), Box::new(chunk));
    }

    pub fn chunk_index(&self, chunk_x: i32, chunk_y: i32) -> u32 {
        let int_to_nat = |i: i32| if i >= 0 {(2 * i) as u32}else{(-2 * i - 1) as u32};
        let xx: u32 = int_to_nat(chunk_x);
        let yy: u32 = int_to_nat(chunk_y);

        // TODO: this multiply is the first thing to overflow if you go out too far
        //          (though you need to go out ~32768 chunks (2^16 / 2)
        return ((xx + yy) * (xx + yy + 1)) / 2 + yy;
    }
    

    pub fn chunk_index_inv(&self, index: u32) -> (i32, i32) {
        let w = (((8 * index + 1) as f32).sqrt() - 1.0).floor() as u32 / 2;
        let t = (w * w + w) / 2;
        let yy = index - t;
        let xx = w - yy;
        let nat_to_int = |i: u32| if i % 2 == 0 {(i/2) as i32}else{-((i/2 + 1) as i32)};
        let x = nat_to_int(xx);
        let y = nat_to_int(yy);

        return (x, y);
    }

    #[profiling::function]
    pub fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks.contains_key(&self.chunk_index(chunk_x, chunk_y))
    }

    #[profiling::function]
    pub fn is_pixel_loaded(&self, x: i64, y: i64) -> bool {
        let chunk_pos = self.pixel_to_chunk_pos(x, y);
        self.is_chunk_loaded(chunk_pos.0, chunk_pos.1)
    }

    #[profiling::function]
    pub fn pixel_to_chunk_pos(&self, x: i64, y: i64) -> (i32, i32) {
        ((x as f64 / CHUNK_SIZE as f64).floor() as i32,
            (y as f64 / CHUNK_SIZE as f64).floor() as i32)
    }

    #[profiling::function]
    pub fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&Box<Chunk>> {
        self.loaded_chunks.get(&self.chunk_index(chunk_x, chunk_y))
    }

    pub fn set(&mut self, x: i64, y: i64, mat: MaterialInstance) -> Result<(), String> {

        let (chunk_x, chunk_y) = self.pixel_to_chunk_pos(x, y);
        if let Some(ch) = self.loaded_chunks.get_mut(&self.chunk_index(chunk_x, chunk_y)) {
            return ch.set((x - chunk_x as i64 * CHUNK_SIZE as i64) as u16, (y - chunk_y as i64 * CHUNK_SIZE as i64) as u16, mat);
        }else{
            return Err("Position is not loaded".to_string());
        }
    }

    pub fn chunk_update_order(&self, chunk_x: i32, chunk_y: i32) -> u8 {
        let yy = (-chunk_y).rem_euclid(2) as u8;
        let xx = chunk_x.rem_euclid(2) as u8;
        return yy * 2 + xx;
    }

    pub fn force_update_chunk(&mut self, chunk_x: i32, chunk_y: i32) {
        if let Some(ch) = self.loaded_chunks.get_mut(&self.chunk_index(chunk_x, chunk_y)) {
            ch.dirty_rect = Some(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));
        }
    }

    #[profiling::function]
    pub fn get_zone(&self, center: (f64, f64), padding: u32) -> Rect {
        let width = self.screen_size.0 as u32 + padding * 2;
        let height = self.screen_size.1 as u32 + padding * 2;
        Rect::new(center.0 as i32 - (width / 2) as i32, center.1 as i32 - (height / 2) as i32, width, height)
    }

    #[profiling::function]
    pub fn get_screen_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, 0)
    }

    #[profiling::function]
    pub fn get_active_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, CHUNK_SIZE.into())
    }

    #[profiling::function]
    pub fn get_load_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, (CHUNK_SIZE * 5).into())
    }

    #[profiling::function]
    pub fn get_unload_zone(&self, center: (f64, f64)) -> Rect {
        self.get_zone(center, (CHUNK_SIZE * 10).into())
    }
}