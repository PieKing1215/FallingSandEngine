use sdl2::{pixels::Color, rect::Rect};

use crate::game::{Game, RenderCanvas, Renderable, TransformStack};

use super::{CHUNK_SIZE, ChunkHandler};

pub struct World {
    pub camera: Camera,
    pub chunk_handler: ChunkHandler,
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

impl World {
    pub fn create() -> Self {
        World {
            camera: Camera {
                x: 0.0,
                y: 0.0,
                scale: 2.0,
            },
            chunk_handler: ChunkHandler::new(),
        }
    }

    pub fn tick(&mut self, tick_time: u32){
        self.chunk_handler.tick(tick_time, &self.camera);
    }
}

impl Renderable for World {
    fn render(&self, canvas : &mut RenderCanvas, transform: &mut TransformStack, _game: &Game) {

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

        self.chunk_handler.loaded_chunks.iter().for_each(|ch| {
            let rc = Rect::new(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);

            if rc.has_intersection(screen_zone) {
                canvas.set_draw_color(Color::RGBA(64, 255, 64, 255));
            }else if rc.has_intersection(active_zone) {
                canvas.set_draw_color(Color::RGBA(255, 255, 64, 255));
            }else if rc.has_intersection(load_zone) {
                canvas.set_draw_color(Color::RGBA(255, 127, 64, 127));
            }else {
                canvas.set_draw_color(Color::RGBA(255, 64, 64, 64));
            }
            canvas.draw_rect(transform.transform_rect(rc)).unwrap();
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

