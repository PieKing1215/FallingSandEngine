use crate::game::world::gen::WorldGenerator;
use sdl2::{pixels::Color, rect::Rect, render::{Canvas, TextureCreator}, video::WindowContext};

use crate::game::{Fonts, Game, RenderCanvas, Renderable, Sdl2Context, TransformStack};

use super::{CHUNK_SIZE, ChunkHandler, gen::{TEST_GENERATOR, TestGenerator}};

pub struct World<'w> {
    pub camera: Camera,
    pub chunk_handler: ChunkHandler<'w, TestGenerator>,
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

impl<'w> World<'w> {
    #[profiling::function]
    pub fn create() -> Self {
        World {
            camera: Camera {
                x: 0.0,
                y: 0.0,
                scale: 2.0,
            },
            chunk_handler: ChunkHandler::new(TEST_GENERATOR),
        }
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, texture_creator: &'w TextureCreator<WindowContext>){
        self.chunk_handler.tick(tick_time, &self.camera);
        self.chunk_handler.update_chunk_graphics(texture_creator);
    }
}

impl Renderable for World<'_> {
    #[profiling::function]
    fn render(&self, canvas : &mut RenderCanvas, transform: &mut TransformStack, sdl: &Sdl2Context, fonts: &Fonts, game: &Game) {

        // draw world

        canvas.set_draw_color(Color::RGBA(255, 127, 255, 255));

        transform.push();
        transform.translate(canvas.window().size().0 as f64 / 2.0 / self.camera.scale, canvas.window().size().1 as f64 / 2.0 / self.camera.scale);
        
        transform.translate(-self.camera.x, -self.camera.y);
        transform.scale(self.camera.scale, self.camera.scale);

        let screen_zone = self.chunk_handler.get_screen_zone(&self.camera);
        let active_zone = self.chunk_handler.get_active_zone(&self.camera);
        let load_zone = self.chunk_handler.get_load_zone(&self.camera);
        let unload_zone = self.chunk_handler.get_unload_zone(&self.camera);

        for x in -10..10 {
            for y in -10..10 {
                let rc = Rect::new(x * CHUNK_SIZE as i32, y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
                canvas.set_draw_color(Color::RGBA(127, 127, 127, 64));
                canvas.draw_rect(transform.transform_rect(rc)).unwrap();
            }
        }

        self.chunk_handler.loaded_chunks.iter().for_each(|(_i, ch)| {
            let culling = false;
            let state_overlay = false;

            let rc = Rect::new(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
            if !culling || rc.has_intersection(screen_zone){
                transform.push();
                transform.translate(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32);
                ch.render(canvas, transform, sdl, fonts, game);
                transform.pop();
            }

            if state_overlay {
                match ch.state {
                    super::ChunkState::Unknown => {
                        canvas.set_draw_color(Color::RGBA(127, 64, 127, 191));
                    },
                    super::ChunkState::NotGenerated => {
                        canvas.set_draw_color(Color::RGBA(127, 127, 127, 191));
                    },
                    super::ChunkState::Generating(stage) => {
                        canvas.set_draw_color(Color::RGBA(64, (stage as f32 / 4.0 * 255.0) as u8, 255, 191));
                    },
                    super::ChunkState::Cached => {
                        canvas.set_draw_color(Color::RGBA(255, 127, 64, 191));
                    },
                    super::ChunkState::Active => {
                        canvas.set_draw_color(Color::RGBA(64, 255, 64, 191));
                    },
                }
                let rect = transform.transform_rect(rc);
                canvas.fill_rect(rect).unwrap();
                canvas.draw_rect(rect).unwrap();
            }
            
            // let ind = self.chunk_handler.chunk_index(ch.chunk_x, ch.chunk_y);
            // let tex = canvas.texture_creator();
            // let txt_sf = fonts.pixel_operator
            //     .render(format!("{}", ind).as_str())
            //     .solid(Color::RGB(255, 255, 255)).unwrap();
            // let txt_tex = tex.create_texture_from_surface(&txt_sf).unwrap();
            // let txt_tex2 = tex.create_texture_from_surface(&txt_sf).unwrap();

            // let aspect = txt_sf.width() as f32 / txt_sf.height() as f32;
            // let mut txt_height = rect.height() as f32 * 0.75;
            // let mut txt_width = (aspect * txt_height as f32) as u32;

            // let max_width = (rect.w as f32 * 0.9) as u32;

            // if txt_width > max_width as u32 {
            //     txt_width = max_width as u32;
            //     txt_height = 1.0 / aspect * txt_width as f32;
            // }

            // let txt_rec = Rect::new(rect.x + rect.w/2 - (txt_width as i32)/2, rect.y, txt_width, txt_height as u32);
            // canvas.copy(&txt_tex, None, Some(txt_rec)).unwrap();

        });

        canvas.set_draw_color(Color::RGBA(0, 255, 0, 255));
        canvas.draw_line(transform.transform_int((-20, 0)), transform.transform_int((20, 0))).unwrap();
        canvas.set_draw_color(Color::RGBA(127, 127, 255, 255));
        canvas.draw_line(transform.transform_int((0, -20)), transform.transform_int((0, 20))).unwrap();

        canvas.set_draw_color(Color::RGBA(255, 0, 0, 127));
        canvas.draw_rect(transform.transform_rect(unload_zone)).unwrap();
        canvas.set_draw_color(Color::RGBA(255, 127, 0, 127));
        canvas.draw_rect(transform.transform_rect(load_zone)).unwrap();
        canvas.set_draw_color(Color::RGBA(255, 255, 0, 127));
        canvas.draw_rect(transform.transform_rect(active_zone)).unwrap();
        canvas.set_draw_color(Color::RGBA(0, 255, 0, 127));
        canvas.draw_rect(transform.transform_rect(screen_zone)).unwrap();

        transform.pop();

        // draw overlay


    }
}

