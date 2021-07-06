use sdl2::pixels::Color;

use crate::game::{Game, Renderable};

pub struct World {

}

impl Renderable for World {
    fn render(&self, canvas : &mut sdl2::render::Canvas<sdl2::video::Window>, _game: &Game) {
        canvas.set_draw_color(Color::RGBA(127, 127, 255, 255));
        canvas.draw_rect(sdl2::rect::Rect::new(10, 10, 20, 20)).unwrap();
    }
}

