use sdl2::rect::Rect;

use crate::game::Game;

use super::Camera;


pub const CHUNK_SIZE: u16 = 128;

pub struct Chunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

pub struct ChunkHandler {
    pub loaded_chunks: Vec<Box<Chunk>>,
    load_queue: Vec<(i32, i32)>,
    /** The size of the "presentable" area (not necessarily the current window size) */
    pub screen_size: (u16, u16),
}

impl ChunkHandler {
    pub fn new() -> Self {
        ChunkHandler {
            loaded_chunks: vec![],
            load_queue: vec![],
            screen_size: (1920, 1080),
        }
    }

    pub fn tick(&mut self, tick_time: u32, camera: &Camera){ // TODO: `camera` should be replaced with like a vec of entities or something

        let load_zone = self.get_load_zone(camera);
        for px in (load_zone.x .. load_zone.x + load_zone.w).step_by(CHUNK_SIZE.into()) {
            for py in (load_zone.y .. load_zone.y + load_zone.h).step_by(CHUNK_SIZE.into()) {
                let chunk_pos = self.pixel_to_chunk_pos(px.into(), py.into());
                self.queue_load_chunk(chunk_pos.0, chunk_pos.1);
            }
        }

        for _ in 0..10 {
            // TODO: don't load queued chunks if they are no longer in range
            if let Some(to_load) = self.load_queue.pop() {
                self.load_chunk(to_load.0, to_load.1);
            }
        }

        let unload_zone = self.get_unload_zone(camera);

        let mut keep_map = vec![true; self.loaded_chunks.len()];
        for i in 0..self.loaded_chunks.len() {
            let ch = &self.loaded_chunks[i];
            let rect = Rect::new(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

            if !rect.has_intersection(unload_zone) {
                self.unload_chunk(ch);
                keep_map[i] = false;
            }
        }

        let mut iter = keep_map.iter();
        self.loaded_chunks.retain(|_| *iter.next().unwrap());

    }

    fn unload_chunk(&self, chunk: &Chunk){
        // write to file, free textures, etc
    }

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

    fn load_chunk(&mut self, chunk_x: i32, chunk_y: i32){
        let chunk = Chunk{
            chunk_x: chunk_x,
            chunk_y: chunk_y,
        };
        self.loaded_chunks.push(Box::new(chunk));
    }

    pub fn is_chunk_loaded(&self, chunk_x: i32, chunk_y: i32) -> bool {
        self.loaded_chunks.iter().any(|ch| ch.chunk_x == chunk_x && ch.chunk_y == chunk_y)
    }

    pub fn is_pixel_loaded(&self, x: i64, y: i64) -> bool {
        let chunk_pos = self.pixel_to_chunk_pos(x, y);
        self.is_chunk_loaded(chunk_pos.0, chunk_pos.1)
    }

    pub fn pixel_to_chunk_pos(&self, x: i64, y: i64) -> (i32, i32) {
        ((x as f64 / CHUNK_SIZE as f64).floor() as i32,
            (y as f64 / CHUNK_SIZE as f64).floor() as i32)
    }

    pub fn get_chunk(&self, chunk_x: i32, chunk_y: i32) -> Option<&Box<Chunk>> {
        self.loaded_chunks.iter().find(|ch| ch.chunk_x == chunk_x && ch.chunk_y == chunk_y)
    }

    pub fn get_zone(&self, camera: &Camera, padding: u32) -> Rect {
        let width = self.screen_size.0 as u32 + padding * 2;
        let height = self.screen_size.1 as u32 + padding * 2;
        Rect::new(camera.x as i32 - (width / 2) as i32, camera.y as i32 - (height / 2) as i32, width, height)
    }

    pub fn get_screen_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, 0)
    }

    pub fn get_active_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, CHUNK_SIZE.into())
    }

    pub fn get_load_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, (CHUNK_SIZE * 3).into())
    }

    pub fn get_unload_zone(&self, camera: &Camera) -> Rect {
        self.get_zone(camera, (CHUNK_SIZE * 10).into())
    }
}