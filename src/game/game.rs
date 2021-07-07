
use super::{Renderer, world::Camera};
use super::world::World;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};


pub struct Game {
    renderer: Option<Renderer>,
    pub world: Option<World>,
    pub tick_time: u32,
    pub frame_count: u32,
    fps_counter: FPSCounter
}

pub struct FPSCounter {
    frames: u16,
    last_update: Instant,
    pub display_value: u16
}

impl Game {
    pub fn new() -> Self {
        Game {
            renderer: None,
            world: Some(World::create()),
            tick_time: 0,
            frame_count: 0,
            fps_counter: FPSCounter {
                frames: 0, 
                last_update: Instant::now(), 
                display_value: 0
            }
        }
    }

    pub fn init(&mut self) -> Result<(), String>  {
        let r = Renderer::create()?;

        self.renderer = Some(r);

        return Ok(());
    }

    pub fn run(&mut self) {
        let mut prev_frame_time = std::time::Instant::now();

        let mut event_pump = self.renderer.as_ref().map(|r| r.sdl.event_pump().unwrap());

        'mainLoop: loop {

            if event_pump.is_some() {
                for event in event_pump.as_mut().unwrap().poll_iter() {
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
            }

            // tick
            let now = std::time::Instant::now();
            if now.saturating_duration_since(prev_frame_time).as_nanos() > 1_000_000_000 / 30 { // 30 ticks per second
                prev_frame_time = now;
                self.tick();
            }

            // render
            if let Some(r) = &self.renderer {
                r.render(self);
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

            // sleep
            ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
        }

        println!("Closing...");
    }

    fn tick(&mut self){
        self.tick_time += 1;

        if let Some(w) = &mut self.world {
            w.tick(self.tick_time);
        }
    }

}