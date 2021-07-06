use sdl2::{pixels::Color, rect::Rect};

use crate::game::{Game, RenderCanvas, Renderable, TransformStack};

pub struct World {
    pub camera: Camera,
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

impl Renderable for World {
    fn render(&self, canvas : &mut RenderCanvas, transform: &mut TransformStack, _game: &Game) {
        canvas.set_draw_color(Color::RGBA(255, 127, 255, 255));
        //canvas.draw_rect(sdl2::rect::Rect::new(10, 10, 20, 20)).unwrap();

        transform.push();
        transform.translate(canvas.window().size().0 as f64 / 2.0 / self.camera.scale, canvas.window().size().1 as f64 / 2.0 / self.camera.scale);
        transform.scale(self.camera.scale, self.camera.scale);
        transform.translate(self.camera.x, self.camera.y);

        for x in -10..10 {
            for y in -10..10 {
                canvas.draw_rect(transform.transform_rect(Rect::new(x * 20, y * 20, 20, 20))).unwrap();
            }
        }

        canvas.set_draw_color(Color::RGBA(0, 255, 0, 255));
        canvas.draw_line(transform.transform_int((-20, 0)), transform.transform_int((20, 0))).unwrap();
        canvas.set_draw_color(Color::RGBA(127, 127, 255, 255));
        canvas.draw_line(transform.transform_int((0, -20)), transform.transform_int((0, 20))).unwrap();

        transform.pop();
    }
}

