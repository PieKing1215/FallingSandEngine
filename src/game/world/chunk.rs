use std::{cell::Cell, collections::HashMap};

use rand::Rng;
use sdl2::{pixels::Color, rect::Rect, render::{TextureCreator, TextureValueError}, surface::Surface, video::WindowContext};

use crate::game::{RenderCanvas, Renderable};

use super::{Camera, MaterialInstance, gen::WorldGenerator};


pub const CHUNK_SIZE: u16 = 128;

pub struct Chunk<'ch> {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub graphics: ChunkGraphics<'ch>,
}

impl<'ch> Chunk<'ch> {
    pub fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            graphics: ChunkGraphics {
                surface: Surface::new(CHUNK_SIZE as u32, CHUNK_SIZE as u32, sdl2::pixels::PixelFormatEnum::ARGB8888).unwrap(),
                texture: None,
                dirty: true,
            },
        }
    }

    pub fn update_graphics(&mut self, texture_creator: &'ch TextureCreator<WindowContext>) -> Result<(), String> {

        self.graphics.update_texture(texture_creator).map_err(|e| e.to_string())?;

        Ok(())
    }
}

impl Renderable for Chunk<'_> {
    fn render(&self, canvas : &mut sdl2::render::Canvas<sdl2::video::Window>, transform: &mut crate::game::TransformStack, sdl: &crate::game::Sdl2Context, fonts: &crate::game::Fonts, game: &crate::game::Game) {
        self.graphics.render(canvas, transform, sdl, fonts, game);
    }
}

#[derive(Clone, Copy)]
pub enum ChunkState {
    Unknown,
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
    pub fn set(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate".to_string())
    }

    pub fn update_texture(&mut self, texture_creator: &'cg TextureCreator<WindowContext>) -> Result<(), TextureValueError> {
        if self.dirty {
            self.texture = Some(self.surface.as_texture(texture_creator)?);
            self.dirty = false;
        }

        Ok(())
    }
}

impl Renderable for ChunkGraphics<'_> {
    fn render(&self, canvas : &mut sdl2::render::Canvas<sdl2::video::Window>, transform: &mut crate::game::TransformStack, sdl: &crate::game::Sdl2Context, fonts: &crate::game::Fonts, game: &crate::game::Game) {
        let chunk_rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

        if let Some(tex) = &self.texture {
            canvas.copy(tex, None, Some(chunk_rect)).unwrap();
        }else{
            canvas.set_draw_color(Color::RGB(127, 0, 0));
            canvas.fill_rect(chunk_rect).unwrap();
        }
    }
}

pub struct ChunkHandler<'a> {
    pub loaded_chunks: HashMap<u32, Box<Chunk<'a>>>,
    load_queue: Vec<(i32, i32)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
    pub generator: Box<dyn WorldGenerator>,
}

impl<'a> ChunkHandler<'a> {
    #[profiling::function]
    pub fn new(generator: Box<dyn WorldGenerator>) -> Self {
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
            for _ in 0..40 {
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
                        }else if num_loaded_this_tick < 8 {
                            // TODO: load from file
                            self.loaded_chunks.get_mut(&key).unwrap().state = ChunkState::Generating(0);
                            self.generator.as_ref().generate(self.loaded_chunks.get_mut(&key).unwrap());
                            num_loaded_this_tick += 1;
                        }
                    },
                    ChunkState::Generating(stage) => {
                        let chunk_x = self.loaded_chunks.get(&key).unwrap().chunk_x;
                        let chunk_y = self.loaded_chunks.get(&key).unwrap().chunk_y;

                        let max_stage = 4;

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

    }

    #[profiling::function]
    fn unload_chunk(&self, chunk: &Chunk){
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