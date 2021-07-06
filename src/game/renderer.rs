
use std::{cell::RefCell};

use sdl2::{pixels::Color, render::{Canvas}, video::Window};

use super::{Game, Renderable, TransformStack};

pub struct Renderer {
    pub sdl: sdl2::Sdl,
    pub canvas: RefCell<Canvas<Window>>
}

impl Renderer {

    pub fn create() -> Result<Self, String> {
        let sdl = sdl2::init()?;
        let sdl_video = sdl.video()?;
    
        let window = Box::new(sdl_video.window("FallingSandRust", 1200, 800)
            .opengl() // allow getting opengl context
            .build()
            .unwrap());
    
        let canvas: RefCell<Canvas<Window>> = RefCell::new(window.into_canvas()
        .index(find_opengl_driver().unwrap()) // explicitly use opengl
        .build().unwrap());

        canvas.borrow_mut().set_draw_color(Color::RGBA(0, 0, 0, 0));
        canvas.borrow_mut().clear();
        canvas.borrow_mut().present();

        return Ok(Renderer {
            sdl: sdl,
            canvas: canvas,
        });
    }

    pub fn render(&self, game: &Game){
        let canvas: &mut Canvas<Window> = &mut self.canvas.borrow_mut();

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();
        
        self.render_internal(canvas, game);

        canvas.present();
    }

    fn render_internal(&self, canvas: &mut Canvas<Window>, game: &Game){
       canvas.set_draw_color(Color::RGBA(255, 0, 0, 255));
       canvas.draw_rect(sdl2::rect::Rect::new(40 + ((game.tick_time as f32 / 5.0).sin() * 20.0) as i32, 30 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0) as i32, 15, 15)).unwrap();

        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        for i in (0..10000).step_by(15) {
            let thru = (i as f32 / 10000.0 * 255.0) as u8;
            let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
            canvas.set_draw_color(Color::RGBA(0, thru, 255-thru, thru2));
            let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;
            canvas.fill_rect(sdl2::rect::Rect::new(75 + (i % 1000) + (((game.frame_count as i32/2 + (i as i32 / 2) - timeshift) as f32 / 100.0).sin() * 50.0) as i32, 0 + (i / 1000)*100 + (((game.frame_count as i32/2 + (i as i32 / 2) - timeshift) as f32 / 100.0).cos() * 50.0) as i32, 20, 20)).unwrap();    
        }

        if let Some(w) = &game.world {
            w.render(canvas, &mut TransformStack::new(), game);
        }
        
    }

}

fn find_opengl_driver() -> Option<u32> {
    for (i, d) in sdl2::render::drivers().enumerate() {
        if d.name == "opengl" {
            return Some(i as u32);
        }
    }
    None
}