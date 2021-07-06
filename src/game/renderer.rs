
use std::cell::RefCell;

use sdl2::{pixels::Color, render::{Canvas, CanvasBuilder}, video::Window};

use super::Game;

pub struct Renderer {
    pub sdl: Option<sdl2::Sdl>,
    pub canvas: Option<RefCell<Canvas<Window>>> // TODO: there's prob a better way to do this
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {
            sdl: None,
            canvas: None
        }
    }

    pub fn init(&mut self) -> Result<(), String> {
        let r_init = sdl2::init();
        if r_init.is_err() {
            return Err(format!("sdl2::init() FAILED {}", r_init.clone().err().unwrap()));
        }
    
        self.sdl = Some(r_init.unwrap());
    
        let r_video = self.sdl.as_ref().unwrap().video();
        if r_video.is_err() {
            return Err(format!("sdl.video() FAILED {}", r_video.unwrap_err()));
        }
    
        let sdl_video = r_video.unwrap();
    
        let window = Box::new(Some(sdl_video.window("FallingSandRust", 1200, 800)
            .opengl() // allow getting opengl context
            .build()
            .unwrap()));
    

        self.canvas = Some(RefCell::new(window.unwrap().into_canvas()
        .index(find_opengl_driver().unwrap()) // explicitly use opengl
        .build().unwrap()));

        self.canvas.as_ref().unwrap().borrow_mut().set_draw_color(Color::RGBA(0, 0, 0, 0));
        self.canvas.as_ref().unwrap().borrow_mut().clear();
        self.canvas.as_ref().unwrap().borrow_mut().present();

        return Ok(());
    }

    pub fn render(&self, game: &Game){
        {
            let mut canvas = self.canvas.as_ref().unwrap().borrow_mut();

            canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
            canvas.clear();
        }
        
        self.render_internal(game);

        {
            let mut canvas = self.canvas.as_ref().unwrap().borrow_mut();
            canvas.present();
        }
    }

    fn render_internal(&self, game: &Game){
        self.canvas.as_ref().unwrap().borrow_mut().set_draw_color(Color::RGBA(255, 0, 0, 255));
        self.canvas.as_ref().unwrap().borrow_mut().draw_rect(sdl2::rect::Rect::new(40 + ((game.tick_time as f32 / 5.0).sin() * 20.0) as i32, 30 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0) as i32, 15, 15)).unwrap();

        self.canvas.as_ref().unwrap().borrow_mut().set_blend_mode(sdl2::render::BlendMode::Blend);
        for i in 0..10000 {
            let thru = (i as f32 / 10000.0 * 255.0) as u8;
            let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
            self.canvas.as_ref().unwrap().borrow_mut().set_draw_color(Color::RGBA(0, thru, 255-thru, thru2));
            let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;
            self.canvas.as_ref().unwrap().borrow_mut().fill_rect(sdl2::rect::Rect::new(75 + (i % 1000) + (((game.frame_count as i32 + (i as i32 / 2) - timeshift) as f32 / 100.0).sin() * 50.0) as i32, 0 + (i / 1000)*100 + (((game.frame_count as i32 + (i as i32 / 2) - timeshift) as f32 / 100.0).cos() * 50.0) as i32, 20, 20)).unwrap();    
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