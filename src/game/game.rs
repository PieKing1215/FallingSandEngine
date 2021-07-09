
use super::{Fonts, Sdl2Context};
use super::{Renderer, world::Camera};
use super::world::World;

use sdl2::Sdl;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;
use std::time::{Duration, Instant};


pub struct Game<'a> {
    pub world: Option<World<'a>>,
    pub tick_time: u32,
    pub frame_count: u32,
    fps_counter: FPSCounter
}

pub struct FPSCounter {
    frames: u16,
    last_update: Instant,
    pub display_value: u16
}

impl<'a, 'b> Game<'a> {
    #[profiling::function]
    pub fn new() -> Self {
        Game {
            world: Some(World::create()),
            tick_time: 0,
            frame_count: 0,
            fps_counter: FPSCounter {
                frames: 0, 
                last_update: Instant::now(), 
                display_value: 0
            },
        }
    }

    // #[profiling::function]
    // pub fn init(&'b mut self, sdl: &'a Sdl2Context) -> Result<(), String>  {
        
    //     let r = Box::new(Renderer::create(&sdl)?);
    //     self.renderer = Some(r);

    //     let rm = self.renderer.as_mut().unwrap();
    //     let pixel_operator2 = sdl.sdl_ttf.load_font("./assets/font/pixel_operator/PixelOperator.ttf", 16).unwrap();
    //     let f = Some(Fonts {
    //         pixel_operator: pixel_operator2,
    //     });
    //     rm.fonts = f;
    //     self.sdl = Some(sdl);

    //     return Ok(());
    // }

    #[profiling::function]
    pub fn run(&mut self, sdl: &Sdl2Context, renderer: Option<&Renderer>, texture_creator: &'a TextureCreator<WindowContext>) {
        let mut prev_frame_time = std::time::Instant::now();

        let mut event_pump = sdl.sdl.event_pump().unwrap();

        'mainLoop: loop {

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'mainLoop
                    },
                    Event::MouseWheel { y, .. } => {
                        if let Some(w) = &mut self.world {
                            w.camera.scale *= 1.0 + 0.1 * y as f64;
                        }
                    },
                    Event::MouseMotion{xrel, yrel, mousestate , ..} => {
                        if mousestate.left() {
                            if let Some(w) = &mut self.world {
                                w.camera.x -= (xrel as f64) / w.camera.scale;
                                w.camera.y -= (yrel as f64) / w.camera.scale;
                            }
                        }
                    },
                    _ => {}
                }
            }

            // tick
            let now = std::time::Instant::now();
            if now.saturating_duration_since(prev_frame_time).as_nanos() > 1_000_000_000 / 30 { // 30 ticks per second
                prev_frame_time = now;
                self.tick(texture_creator);
            }

            // render
            if let Some(r) = renderer {
                r.render(sdl, self);
                self.frame_count += 1;
                self.fps_counter.frames += 1;
                if now.saturating_duration_since(self.fps_counter.last_update).as_millis() >= 1000 {
                    self.fps_counter.display_value = self.fps_counter.frames;
                    self.fps_counter.frames = 0;
                    self.fps_counter.last_update = now;
                    let set = r.canvas.borrow_mut().window_mut().set_title(format!("FallingSandRust ({} FPS)", self.fps_counter.display_value).as_str());
                    if set.is_err() {
                        eprintln!("Failed to set window title.");
                    }
                }
            }

            profiling::finish_frame!();
            // sleep
            ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
        }

        println!("Closing...");
    }

    #[profiling::function]
    fn tick(&mut self, texture_creator: &'a TextureCreator<WindowContext>){
        self.tick_time += 1;

        if let Some(w) = &mut self.world {
            w.tick(self.tick_time, texture_creator);
        }
    }

}