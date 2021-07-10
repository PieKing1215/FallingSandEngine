use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use lazy_static::lazy_static;
use sdl2::{pixels::Color, rect::Rect, render::{TextureCreator, TextureValueError}, surface::Surface, video::WindowContext};
use tokio::runtime::Runtime;

use crate::game::{Renderable, world::TEST_MATERIAL};

use super::{Camera, MaterialInstance, gen::WorldGenerator};


pub const CHUNK_SIZE: u16 = 128;

pub struct Chunk<'ch> {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub graphics: Box<ChunkGraphics<'ch>>,
}

impl<'ch> Chunk<'ch> {
    pub fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            graphics: Box::new(ChunkGraphics {
                surface: Surface::new(CHUNK_SIZE as u32, CHUNK_SIZE as u32, sdl2::pixels::PixelFormatEnum::ARGB8888).unwrap(),
                texture: None,
                dirty: true,
            }),
        }
    }

    pub fn refresh(&mut self){
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                self.graphics.set(x, y, self.pixels.unwrap()[(x + y * CHUNK_SIZE) as usize].color).unwrap();
            }
        }
    }

    pub fn update_graphics(&mut self, texture_creator: &'ch TextureCreator<WindowContext>) -> Result<(), String> {

        self.graphics.update_texture(texture_creator).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[profiling::function]
    pub fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {

            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                px[i] = mat;
                self.graphics.set(x, y, px[i].color)?;

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

impl Renderable for Chunk<'_> {
    fn render(&self, canvas : &mut sdl2::render::Canvas<sdl2::video::Window>, transform: &mut crate::game::TransformStack, sdl: &crate::game::Sdl2Context, fonts: &crate::game::Fonts, game: &crate::game::Game) {
        self.graphics.render(canvas, transform, sdl, fonts, game);
    }
}

#[derive(Clone, Copy)]
pub enum ChunkState {
    NotGenerated,
    Generating(u8), // stage
    Cached,
    Active,
}

pub struct ChunkGraphics<'cg> {
    surface: sdl2::surface::Surface<'cg>,
    texture: Option<sdl2::render::Texture<'cg>>,
    dirty: bool,
}

impl<'cg> ChunkGraphics<'cg> {
    #[profiling::function]
    pub fn set(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    pub fn update_texture(&mut self, texture_creator: &'cg TextureCreator<WindowContext>) -> Result<(), TextureValueError> {
        if self.dirty {
            self.texture = Some(self.surface.as_texture(texture_creator)?);
            self.dirty = false;
        }

        Ok(())
    }

    #[profiling::function]
    pub fn replace(&mut self, mut colors: [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]){
        let sf = Surface::from_data(&mut colors, CHUNK_SIZE as u32, CHUNK_SIZE as u32, self.surface.pitch(), self.surface.pixel_format_enum()).unwrap();
        sf.blit(None, &mut self.surface, None).unwrap();
        self.dirty = true;
    }
}

impl Renderable for ChunkGraphics<'_> {
    fn render(&self, canvas : &mut sdl2::render::Canvas<sdl2::video::Window>, transform: &mut crate::game::TransformStack, _sdl: &crate::game::Sdl2Context, _fonts: &crate::game::Fonts, _game: &crate::game::Game) {
        let chunk_rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

        if let Some(tex) = &self.texture {
            canvas.copy(tex, None, Some(chunk_rect)).unwrap();
        }else{
            canvas.set_draw_color(Color::RGB(127, 0, 0));
            canvas.fill_rect(chunk_rect).unwrap();
        }
    }
}

pub struct ChunkHandler<'a, T: WorldGenerator + Copy + Send + Sync + 'static> {
    pub loaded_chunks: HashMap<u32, Box<Chunk<'a>>>,
    load_queue: Vec<(i32, i32)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: T,
}

impl<'a, T: WorldGenerator + Copy + Send + Sync + 'static> ChunkHandler<'a, T> {
    #[profiling::function]
    pub fn new(generator: T) -> Self {
        ChunkHandler {
            loaded_chunks: HashMap::new(),
            load_queue: vec![],
            screen_size: (1920, 1080),
            generator
        }
    }

    pub fn update_chunk_graphics(&mut self, texture_creator: &'a TextureCreator<WindowContext>){
        let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
        for i in 0..keys.len() {
            let key = keys[i];
            self.loaded_chunks.get_mut(&key).unwrap().update_graphics(texture_creator).unwrap();
        }
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, camera: &Camera){ // TODO: `camera` should be replaced with like a vec of entities or something
        
        let unload_zone = self.get_unload_zone(camera);
        let load_zone = self.get_load_zone(camera);
        let active_zone = self.get_active_zone(camera);
        let screen_zone = self.get_screen_zone(camera);
        
        {
            profiling::scope!("queue chunk loading");
            for px in (load_zone.x .. load_zone.x + load_zone.w).step_by(CHUNK_SIZE.into()) {
                for py in (load_zone.y .. load_zone.y + load_zone.h).step_by(CHUNK_SIZE.into()) {
                    let chunk_pos = self.pixel_to_chunk_pos(px.into(), py.into());
                    self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
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
                        if !rect.has_intersection(unload_zone) {
                            self.unload_chunk(&self.loaded_chunks.get(&key).unwrap());
                            keep_map[i] = false;
                        }else if rect.has_intersection(active_zone) {
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
                            }
                        }
                    },
                    ChunkState::Active => {
                        if !rect.has_intersection(active_zone) {
                            self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Cached;
                        }
                    }
                    _ => {},
                }
            }

            let mut iter = keep_map.iter();
            self.loaded_chunks.retain(|_, _| *iter.next().unwrap());
        }

        if tick_time % 2 == 0 {
            profiling::scope!("chunk update B");

            let mut num_loaded_this_tick = 0;

            let mut keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
            keys.sort_by(|a, b| {
                let c1_x = self.loaded_chunks.get(a).unwrap().chunk_x * CHUNK_SIZE as i32;
                let c1_y = self.loaded_chunks.get(a).unwrap().chunk_y * CHUNK_SIZE as i32;
                let c2_x = self.loaded_chunks.get(b).unwrap().chunk_x * CHUNK_SIZE as i32;
                let c2_y = self.loaded_chunks.get(b).unwrap().chunk_y * CHUNK_SIZE as i32;

                let d1_x = (camera.x as i32 - c1_x).abs();
                let d1_y = (camera.y as i32 - c1_y).abs();
                let d1 = d1_x + d1_y;

                let d2_x = (camera.x as i32 - c2_x).abs();
                let d2_y = (camera.y as i32 - c2_y).abs();
                let d2 = d2_x + d2_y;

                d1.cmp(&d2)
            });
            let mut to_exec = vec![];
            for i in 0..keys.len() {
                let key = keys[i];
                let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                let rect = Rect::new(self.loaded_chunks.get(&key).unwrap().chunk_x * CHUNK_SIZE as i32, self.loaded_chunks.get(&key).unwrap().chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

                match state {
                    ChunkState::NotGenerated => {
                        if !rect.has_intersection(unload_zone) {

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
                        if !rect.has_intersection(unload_zone) {
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

                            if !rect.has_intersection(unload_zone) {
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

        if tick_time % 30 == 0 {
            profiling::scope!("chunk simulate");
            let keys = self.loaded_chunks.keys().clone().map(|i| *i).collect::<Vec<u32>>();
            for tick_phase in 0..9 {
                for i in 0..keys.len() {
                    let key = keys[i];
                    let state = self.loaded_chunks.get(&key).unwrap().state; // copy
                    let ch_pos = (self.loaded_chunks.get(&key).unwrap().chunk_x, self.loaded_chunks.get(&key).unwrap().chunk_y);
                    if self.chunk_update_order(ch_pos.0, ch_pos.1) == tick_phase {
                        match state {
                            ChunkState::Active => {
                                profiling::scope!("iter");
                                let ch00 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
                                let ch10 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
                                let ch20 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 - 1)).unwrap().pixels.as_ref().unwrap();
                                let ch01 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
                                let ch11 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
                                let ch21 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 0)).unwrap().pixels.as_ref().unwrap();
                                let ch02 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 - 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
                                let ch12 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 0, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
                                let ch22 = self.loaded_chunks.get(&self.chunk_index(ch_pos.0 + 1, ch_pos.1 + 1)).unwrap().pixels.as_ref().unwrap();
                                let arr = [
                                    ch00, ch10, ch20, 
                                    ch01, ch11, ch21, 
                                    ch02, ch12, ch22 ];

                                let diff = self.simulate_chunk(arr);

                                for i in 0..9 {
                                    if diff[i].len() > 0 {
                                        let rel_ch_x = (i % 3) as i32 - 1;
                                        let rel_ch_y = (i / 3) as i32 - 1;
                                        self.loaded_chunks.get_mut(&self.chunk_index(ch_pos.0 + rel_ch_x, ch_pos.1 + rel_ch_y)).unwrap()
                                            .apply_diff(&diff[i]);
                                    }
                                }
                            },
                            _ => {},
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
    fn simulate_chunk(&self, old_pixels: [&[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]; 9]) -> [Vec<(u16, u16, MaterialInstance)>; 9] {
        let mut ret = [vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
        
        // let mut pix = |x: i32, y: i32| {
        //     let size = CHUNK_SIZE as i32;
        //     // if x < -size || y < -size || x >= 2 * size || y >= 2 * size {
        //     //     return Err("Chunk index out of bounds");
        //     // }
        //     let rel_chunk_x = (x as f32 / CHUNK_SIZE as f32).floor() as i8;
        //     let rel_chunk_y = (y as f32 / CHUNK_SIZE as f32).floor() as i8;
            
        //     let chunk_px_x = x.rem_euclid(size) as usize;
        //     let chunk_px_y = y.rem_euclid(size) as usize;

        //     &mut pixels[(rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3][chunk_px_x + chunk_px_y * CHUNK_SIZE as usize]

        //     // return Ok(());
        // };

        let index_helper = |x: i32, y: i32| {
            let size = CHUNK_SIZE as i32;
            // if x < -size || y < -size || x >= 2 * size || y >= 2 * size {
            //     return Err("Chunk index out of bounds");
            // }
            let rel_chunk_x = (x as f32 / CHUNK_SIZE as f32).floor() as i8;
            let rel_chunk_y = (y as f32 / CHUNK_SIZE as f32).floor() as i8;
            
            let chunk_px_x = x.rem_euclid(size) as u16;
            let chunk_px_y = y.rem_euclid(size) as u16;

            ((rel_chunk_x + 1) as usize + (rel_chunk_y + 1) as usize * 3, chunk_px_x, chunk_px_y)

            // return Ok(());
        };

        let mut set_pixel = |x: i32, y: i32, mat: MaterialInstance| {
            let i = index_helper(x, y);
            ret[i.0].push((i.1, i.2, mat));
        };

        const CENTER_CHUNK: usize = 4;

        for y in (0..CHUNK_SIZE as i32).rev() {
            for x in 0..CHUNK_SIZE as i32 {
                if old_pixels[CENTER_CHUNK][(x + y * CHUNK_SIZE as i32) as usize].color.g == 255 {
                    set_pixel(x, y, MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: crate::game::world::PhysicsType::Solid,
                        color: Color::RGB(0, 0, 255),
                    });
                }else if old_pixels[CENTER_CHUNK][(x + y * CHUNK_SIZE as i32) as usize].color.b == 255 {
                    set_pixel(x, y, MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: crate::game::world::PhysicsType::Solid,
                        color: Color::RGB(0, 255, 0),
                    });
                }
            }
        }
        
        ret
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

    pub fn chunk_update_order(&self, chunk_x: i32, chunk_y: i32) -> u8 {
        let yy = (-chunk_y).rem_euclid(3) as u8;
        let xx = chunk_x.rem_euclid(3) as u8;
        return yy * 3 + xx;
    }

    #[profiling::function]
    pub fn get_zone(&self, camera: &Camera, padding: u32) -> Rect {
        let width = self.screen_size.0 as u32 + padding * 2;
        let height = self.screen_size.1 as u32 + padding * 2;
        Rect::new(camera.x as i32 - (width / 2) as i32, camera.y as i32 - (height / 2) as i32, width, height)
    }

    #[profiling::function]
    pub fn get_screen_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, 0)
    }

    #[profiling::function]
    pub fn get_active_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, CHUNK_SIZE.into())
    }

    #[profiling::function]
    pub fn get_load_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, (CHUNK_SIZE * 8).into())
    }

    #[profiling::function]
    pub fn get_unload_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, (CHUNK_SIZE * 12).into())
    }
}