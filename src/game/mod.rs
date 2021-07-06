mod game;
mod renderer;
mod world;

pub use game::*;
pub use renderer::*;

use sdl2::{render::Canvas, video::Window};

trait Renderable {
    fn render(&self, canvas : &mut Canvas<Window>, game: &Game);
}