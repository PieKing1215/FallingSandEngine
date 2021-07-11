
use crate::game::world::{CHUNK_SIZE, MaterialInstance};

use super::{Sdl2Context, Settings};
use super::Renderer;
use super::world::World;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse;
use sdl2::render::TextureCreator;
use sdl2::video::{FullscreenType, SwapInterval, WindowContext};
use std::time::{Duration, Instant};


pub struct Game<'a> {
    pub world: Option<World<'a>>,
    pub tick_time: u32,
    pub frame_count: u32,
    fps_counter: FPSCounter,
    pub settings: Settings,
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
            settings: Settings::default(),
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
    pub fn run(&mut self, sdl: &Sdl2Context, mut renderer: Option<&mut Renderer>, texture_creator: &'a TextureCreator<WindowContext>) {
        let mut prev_frame_time = std::time::Instant::now();

        let mut event_pump = sdl.sdl.event_pump().unwrap();

        let mut shift_key = false;

        let mut last_frame = Instant::now();

        let mut do_tick_next = false;
        'mainLoop: loop {

            for event in event_pump.poll_iter() {
                if let Some(r) = &mut renderer {
                    r.imgui_sdl2.handle_event(&mut r.imgui, &event);
                    if r.imgui_sdl2.ignore_event(&event) { continue; }
                }

                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'mainLoop
                    },
                    Event::KeyDown { keycode: Some(Keycode::F11), .. } => {
                        if let Some(ref r) = renderer {
                            let fs = r.canvas.borrow().window().fullscreen_state();
                            if fs == FullscreenType::Off {
                                r.canvas.borrow_mut().window_mut().set_fullscreen(FullscreenType::Desktop).unwrap();
                            }else{
                                r.canvas.borrow_mut().window_mut().set_fullscreen(FullscreenType::Off).unwrap();
                            }
                        }
                    },
                    Event::KeyDown { keycode: Some(Keycode::RShift | Keycode::LShift), .. } => {
                        shift_key = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::RShift | Keycode::LShift), .. } => {
                        shift_key = false;
                    },
                    Event::MouseWheel { y, .. } => {
                        if let Some(w) = &mut self.world {
                            if shift_key {
                                let mut v = w.camera.scale + 0.1 * y as f64;
                                if y > 0 {
                                    v = v.ceil();
                                }else {
                                    v = v.floor();
                                }

                                v = v.clamp(1.0, 10.0);
                                w.camera.scale = v;
                            }else{
                                w.camera.scale = (w.camera.scale * (1.0 + 0.1 * y as f64)).clamp(0.01, 10.0);
                            }
                        }
                    },
                    Event::MouseButtonDown{mouse_btn: sdl2::mouse::MouseButton::Right, x, y, ..} => {
                        if let Some(w) = &mut self.world {
                            if let Some(ref r) = renderer {
                                let world_x = w.camera.x + (x as f64 - r.canvas.borrow().window().size().0 as f64 / 2.0) / w.camera.scale;
                                let world_y = w.camera.y + (y as f64 - r.canvas.borrow().window().size().1 as f64 / 2.0) / w.camera.scale;
                                let (chunk_x, chunk_y) = w.chunk_handler.pixel_to_chunk_pos(world_x as i64, world_y as i64);
                                w.chunk_handler.force_update_chunk(chunk_x, chunk_y);
                            }
                        }
                    },
                    Event::MouseMotion{xrel, yrel, mousestate , x, y, ..} => {
                        if mousestate.left() {
                            if let Some(w) = &mut self.world {
                                w.camera.x -= (xrel as f64) / w.camera.scale;
                                w.camera.y -= (yrel as f64) / w.camera.scale;
                            }
                        }else if mousestate.middle() {
                            if let Some(w) = &mut self.world {
                                if let Some(ref r) = renderer {
                                    let world_x = w.camera.x + (x as f64 - r.canvas.borrow().window().size().0 as f64 / 2.0) / w.camera.scale;
                                    let world_y = w.camera.y + (y as f64 - r.canvas.borrow().window().size().1 as f64 / 2.0) / w.camera.scale;

                                    for xx in -3..=3 {
                                        for yy in -3..=3 {
                                            match w.chunk_handler.set(world_x as i64 + xx, world_y as i64 + yy, MaterialInstance::air()) {
                                                Ok(_) => {},
                                                Err(_) => {},
                                            };
                                        }
                                    }

                                }
                            }
                        }
                    },
                    _ => {}
                }
            }

            let now = std::time::Instant::now();

            if let Some(r) = &mut renderer {
                r.imgui_sdl2.prepare_frame(r.imgui.io_mut(), &r.canvas.borrow().window(), &event_pump.mouse_state());
                let delta = now - last_frame;
                let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
                last_frame = now;
                r.imgui.io_mut().delta_time = delta_s;
            }

            // tick
            if do_tick_next {
                prev_frame_time = now;
                self.tick(texture_creator);
            }
            do_tick_next = now.saturating_duration_since(prev_frame_time).as_nanos() > 1_000_000_000 / 30; // 30 ticks per second

            // render
            if let Some(r) = &mut renderer {
                profiling::scope!("rendering");

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
            if !do_tick_next {
                ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
            }
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