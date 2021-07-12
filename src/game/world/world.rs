use crate::game::world::gen::WorldGenerator;
use sdl2::{pixels::Color, rect::Rect, render::TextureCreator, video::WindowContext};

use crate::game::{Fonts, Game, RenderCanvas, Renderable, Sdl2Context, TransformStack};

use super::{CHUNK_SIZE, ChunkHandler, MaterialInstance, gen::{TEST_GENERATOR, TestGenerator}};

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

        // let clip = canvas.clip_rect();
        // if game.settings.cull_chunks {
        //     canvas.set_clip_rect(transform.transform_rect(screen_zone));
        // }

        self.chunk_handler.loaded_chunks.iter().for_each(|(_i, ch)| {
            let rc = Rect::new(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
            if !game.settings.cull_chunks || rc.has_intersection(screen_zone){
                transform.push();
                transform.translate(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32);
                ch.render(canvas, transform, sdl, fonts, game);

                if game.settings.draw_chunk_dirty_rects {
                    if let Some(dr) = ch.dirty_rect {
                        let rect = transform.transform_rect(dr);
                        canvas.set_draw_color(Color::RGBA(255, 64, 64, 127));
                        canvas.fill_rect(rect).unwrap();
                        canvas.draw_rect(rect).unwrap();
                    }
                    if ch.graphics.was_dirty {
                        let rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));
                        canvas.set_draw_color(Color::RGBA(255, 255, 64, 127));
                        canvas.fill_rect(rect).unwrap();
                        canvas.draw_rect(rect).unwrap();
                    }
                }

                transform.pop();
            }

            if game.settings.draw_chunk_state_overlay {
                let rect = transform.transform_rect(rc);

                let alpha: u8 = (game.settings.draw_chunk_state_overlay_alpha * 255.0) as u8;
                match ch.state {
                    super::ChunkState::NotGenerated => {
                        canvas.set_draw_color(Color::RGBA(127, 127, 127, alpha));
                    },
                    super::ChunkState::Generating(stage) => {
                        canvas.set_draw_color(Color::RGBA(64, (stage as f32 / self.chunk_handler.generator.max_gen_stage() as f32 * 255.0) as u8, 255, alpha));
                    },
                    super::ChunkState::Cached => {
                        canvas.set_draw_color(Color::RGBA(255, 127, 64, alpha));
                    },
                    super::ChunkState::Active => {
                        canvas.set_draw_color(Color::RGBA(64, 255, 64, alpha));
                    },
                }
                canvas.fill_rect(rect).unwrap();
                canvas.draw_rect(rect).unwrap();
            
                // let ind = self.chunk_handler.chunk_index(ch.chunk_x, ch.chunk_y);
                // let ind = self.chunk_handler.chunk_update_order(ch.chunk_x, ch.chunk_y);
                // let tex = canvas.texture_creator();
                // let txt_sf = fonts.pixel_operator
                //     .render(format!("{}", ind).as_str())
                //     .solid(Color::RGB(255, 255, 255)).unwrap();
                // let txt_tex = tex.create_texture_from_surface(&txt_sf).unwrap();
    
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
            }

        });

        // canvas.set_clip_rect(clip);
        
        if game.settings.draw_chunk_grid {
            for x in -10..10 {
                for y in -10..10 {
                    let rcx = x + (self.camera.x / CHUNK_SIZE as f64) as i32;
                    let rcy = y + (self.camera.y / CHUNK_SIZE as f64) as i32;
                    let rc = Rect::new(rcx * CHUNK_SIZE as i32, rcy * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
                    canvas.set_draw_color(Color::RGBA(64, 64, 64, 127));
                    canvas.draw_rect(transform.transform_rect(rc)).unwrap();
                }
            }
        }

        if game.settings.draw_origin {
            let len = 16;
            canvas.set_draw_color(Color::RGBA(0, 0, 0, 127));
            let origin = transform.transform_int((0, 0));
            canvas.fill_rect(Rect::new(origin.0 - len - 2, origin.1 - 1, (len * 2 + 4) as u32, 3)).unwrap();
            canvas.fill_rect(Rect::new(origin.0 - 1, origin.1 - len - 2, 3, (len * 2 + 4) as u32)).unwrap();

            canvas.set_draw_color(Color::RGBA(255, 0, 0, 255));
            canvas.draw_line((origin.0 - len, origin.1), (origin.0 + len, origin.1)).unwrap();
            canvas.set_draw_color(Color::RGBA(0, 255, 0, 255));
            canvas.draw_line((origin.0, origin.1 - len), (origin.0, origin.1 + len)).unwrap();
        }

        if game.settings.draw_load_zones {
            canvas.set_draw_color(Color::RGBA(255, 0, 0, 127));
            canvas.draw_rect(transform.transform_rect(unload_zone)).unwrap();
            canvas.set_draw_color(Color::RGBA(255, 127, 0, 127));
            canvas.draw_rect(transform.transform_rect(load_zone)).unwrap();
            canvas.set_draw_color(Color::RGBA(255, 255, 0, 127));
            canvas.draw_rect(transform.transform_rect(active_zone)).unwrap();
            canvas.set_draw_color(Color::RGBA(0, 255, 0, 127));
            canvas.draw_rect(transform.transform_rect(screen_zone)).unwrap();
        }

        transform.pop();

        // draw overlay


    }
}

