use sdl2::rect::Rect;

use crate::game::{common::world::{CHUNK_SIZE, Chunk, ChunkState, material::MaterialInstance}};

pub struct ServerChunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub dirty_rect: Option<Rect>,
    pub pixel_data: [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4],
}

impl<'ch> Chunk for ServerChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            dirty_rect: None,
            pixel_data: [0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)],
        }
    }

    fn get_chunk_x(&self) -> i32 {
        self.chunk_x
    }

    fn get_chunk_y(&self) -> i32 {
        self.chunk_y
    }

    fn get_state(&self) -> ChunkState {
        self.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.state = state;
    }

    fn get_dirty_rect(&self) -> Option<Rect> {
        self.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect>) {
        self.dirty_rect = rect;
    }

    fn refresh(&mut self){
    }

    // #[profiling::function]
    fn update_graphics(&mut self) -> Result<(), String> {
        Ok(())
    }

    // #[profiling::function] // huge performance impact
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {

            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                px[i] = mat;

                self.dirty_rect = Some(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    fn apply_diff(&mut self, diff: &Vec<(u16, u16, MaterialInstance)>) {
        diff.iter().for_each(|(x, y, mat)| {
            self.set(*x, *y, *mat).unwrap(); // TODO: handle this Err
        });
    }

    fn set_pixels(&mut self, pixels: &[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]) {
        self.pixels = Some(*pixels);
    }

    fn get_pixels_mut(&mut self) -> &mut Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]> {
        &mut self.pixels
    }

    fn set_pixel_colors(&mut self, _colors: &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]) {
    }

    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.pixel_data
    }

    fn mark_dirty(&mut self) {
    }
}