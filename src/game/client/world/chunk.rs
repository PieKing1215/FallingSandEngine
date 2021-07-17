use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::GPUTarget;

use crate::game::{client::render::{Fonts, Renderable, Sdl2Context, TransformStack}, common::world::{CHUNK_SIZE, Chunk, ChunkGraphics}};



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