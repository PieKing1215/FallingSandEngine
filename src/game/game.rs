
use super::Renderer;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

pub struct Game {
    renderer: Option<Renderer>,
    pub tick_time: u32,
    pub frame_count: u32,
    fpsCounter: FPSCounter
}

pub struct FPSCounter {
    frames: u16,
    last_update: Instant,
    pub display_value: u16
}

impl Game {
    pub fn new() -> Game {
        Game {
            renderer: None,
            tick_time: 0,
            frame_count: 0,
            fpsCounter: FPSCounter {
                frames: 0, 
                last_update: Instant::now(), 
                display_value: 0
            }
        }
    }

    pub fn init(&mut self) -> Result<(), String>  {
        self.renderer = Some(Renderer::new());

        if let Some(r) = &mut self.renderer {
            let initted = r.init();
            if initted.is_err() {
                return initted;
            }
        };



        return Ok(());
    }

    pub fn run(&mut self) {
        let mut prev_frame_time = std::time::Instant::now();

        let mut event_pump = self.renderer.as_ref().map(|r| r.sdl.as_ref().unwrap().event_pump().unwrap());

        'mainLoop: loop {

            if event_pump.is_some() {
                for event in event_pump.as_mut().unwrap().poll_iter() {
                    match event {
                        Event::Quit {..} |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                            break 'mainLoop
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
                self.fpsCounter.frames += 1;
                if now.saturating_duration_since(self.fpsCounter.last_update).as_millis() >= 1000 {
                    self.fpsCounter.display_value = self.fpsCounter.frames;
                    self.fpsCounter.frames = 0;
                    self.fpsCounter.last_update = now;
                    let set = r.canvas.as_ref().unwrap().borrow_mut().window_mut().set_title(format!("FallingSandRust ({} FPS)", self.fpsCounter.display_value).as_str());
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
    }

}