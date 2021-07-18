use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem, GPUTarget, sys::{GPU_FilterEnum, GPU_FormatEnum}};

use crate::game::{client::render::{Fonts, Renderable, Sdl2Context, TransformStack}, common::world::{CHUNK_SIZE, Chunk}};

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

impl Renderable for Chunk {
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