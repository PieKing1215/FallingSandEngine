
use std::convert::TryInto;

use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem, GPUTarget, sys::{GPU_FilterEnum, GPU_FormatEnum}};

use crate::game::{client::render::{Fonts, Renderable, Sdl2Context, TransformStack}, common::world::{CHUNK_SIZE, Chunk, ChunkHandler, ChunkState, gen::WorldGenerator, material::MaterialInstance}};

pub struct ClientChunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    pub graphics: Box<ChunkGraphics>,
    pub dirty_rect: Option<Rect>,
}

impl<'ch> Chunk for ClientChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
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
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                self.graphics.set(x, y, self.pixels.unwrap()[(x + y * CHUNK_SIZE) as usize].color).unwrap();
            }
        }
    }

    // #[profiling::function]
    fn update_graphics(&mut self) -> Result<(), String> {
        
        self.graphics.was_dirty = self.graphics.dirty;

        self.graphics.update_texture().map_err(|e| "ChunkGraphics::update_texture failed.")?;

        Ok(())
    }

    // #[profiling::function] // huge performance impact
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
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

    fn get_pixels(&self) -> &Option<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]> {
        &self.pixels
    }

    fn set_pixel_colors(&mut self, colors: &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]) {
        self.graphics.replace(*colors);
    }

    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.graphics.pixel_data
    }

    fn get_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.graphics.pixel_data
    }

    fn mark_dirty(&mut self) {
        self.graphics.dirty = true;
    }
}

pub struct ChunkGraphics {
    pub texture: Option<GPUImage>,
    pub pixel_data: [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4],
    pub dirty: bool,
    pub was_dirty: bool,
}

impl<'cg> ChunkGraphics {
    // #[profiling::function] // huge performance impact
    pub fn set(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            // self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            let i = (x + y * CHUNK_SIZE) as usize;
            self.pixel_data[i * 4 + 0] = color.r;
            self.pixel_data[i * 4 + 1] = color.g;
            self.pixel_data[i * 4 + 2] = color.b;
            self.pixel_data[i * 4 + 3] = color.a;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    // #[profiling::function]
    pub fn update_texture(&mut self) -> Result<(), ()> {
        if self.dirty {
            if self.texture.is_none() {
                self.texture = Some(GPUSubsystem::create_image(CHUNK_SIZE, CHUNK_SIZE, GPU_FormatEnum::GPU_FORMAT_RGBA));
                self.texture.as_mut().unwrap().set_image_filter(GPU_FilterEnum::GPU_FILTER_NEAREST);
            }
            self.texture.as_mut().unwrap().update_image_bytes(None as Option<GPURect>, &self.pixel_data, (CHUNK_SIZE * 4).into());
            self.dirty = false;
        }

        Ok(())
    }

    #[profiling::function]
    pub fn replace(&mut self, colors: [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]){
        // let sf = Surface::from_data(&mut colors, CHUNK_SIZE as u32, CHUNK_SIZE as u32, self.surface.pitch(), self.surface.pixel_format_enum()).unwrap();
        // sf.blit(None, &mut self.surface, None).unwrap();
        self.pixel_data = colors;
        self.dirty = true;
    }
}

impl Renderable for ClientChunk {
    fn render(&self, canvas : &mut GPUTarget, transform: &mut TransformStack, sdl: &Sdl2Context, fonts: &Fonts) {
        self.graphics.render(canvas, transform, sdl, fonts);
    }
}

impl Renderable for ChunkGraphics {
    fn render(&self, target : &mut GPUTarget, transform: &mut TransformStack, _sdl: &Sdl2Context, _fonts: &Fonts) {
        let chunk_rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));

        if let Some(tex) = &self.texture {
            tex.blit_rect(None, target, Some(chunk_rect));
        }else{
            target.rectangle_filled2(chunk_rect, Color::RGB(127, 0, 0));
        }
    }
}

impl<T: WorldGenerator + Copy + Send + Sync + 'static> ChunkHandler<T, ClientChunk> {
    pub fn sync_chunk(&mut self, chunk_x: i32, chunk_y: i32, pixels: Vec<MaterialInstance>, colors: Vec<u8>) -> Result<(), String>{
        if pixels.len() != (CHUNK_SIZE * CHUNK_SIZE) as usize {
            return Err(format!("pixels Vec is the wrong size: {} (expected {})", pixels.len(), CHUNK_SIZE * CHUNK_SIZE));
        }

        if colors.len() != CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4 {
            return Err(format!("colors Vec is the wrong size: {} (expected {})", colors.len(), CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4));
        }

        if let Some(chunk) = self.loaded_chunks.get_mut(&self.chunk_index(chunk_x, chunk_y)) {
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
        }else{
            let mut chunk: ClientChunk = Chunk::new_empty(chunk_x, chunk_y);
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
            self.loaded_chunks.insert(self.chunk_index(chunk_x, chunk_y), Box::new(chunk));
        }

        Ok(())
    }
}