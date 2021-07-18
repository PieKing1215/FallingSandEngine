
use crate::game::common::world::material::MaterialInstance;

use crate::game::common::Settings;
use crate::game::client::render::Sdl2Context;
use crate::game::client::render::Renderer;
use crate::game::common::world::World;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::sys::SDL_WindowFlags;
use sdl2::video::{FullscreenType, SwapInterval};
use sdl_gpu::GPUSubsystem;
use sysinfo::{Pid, ProcessExt, SystemExt};
use std::time::{Duration, Instant};

use super::client::Client;
use super::client::world::ClientWorld;


pub struct Game {
    pub world: Option<World>,
    pub tick_time: u32,
    pub frame_count: u32,
    pub fps_counter: FPSCounter,
    pub process_stats: ProcessStats,
    pub settings: Settings,
    pub client: Option<Client>,
}

pub struct ProcessStats {
    pub cpu_usage: Option<f32>,
    pub memory: Option<u64>,
}

pub struct FPSCounter {
    frames: u16,
    last_update: Instant,
    pub display_value: u16,
    pub frame_times: [f32; 200],
    pub tick_times: [f32; 200],
    pub tick_lqf_times: [f32; 200],
}

impl<'a, 'b> Game {
    #[profiling::function]
    pub fn new() -> Self {
        Game {
            world: Some(World::create()),
            tick_time: 0,
            frame_count: 0,
            fps_counter: FPSCounter {
                frames: 0, 
                last_update: Instant::now(), 
                display_value: 0,
                frame_times: [0.0; 200],
                tick_times: [0.0; 200],
                tick_lqf_times: [0.0; 200],
            },
            process_stats: ProcessStats {
                cpu_usage: None,
                memory: None,
            },
            settings: Settings::default(),
            client: None,
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
    pub fn run(&mut self, sdl: &Sdl2Context, mut renderer: Option<&mut Renderer>) {
        let mut prev_tick_time = std::time::Instant::now();
        let mut prev_tick_lqf_time = std::time::Instant::now();

        let mut event_pump = sdl.sdl.event_pump().unwrap();

        let mut shift_key = false;

        let mut last_frame = Instant::now();
        let mut counter_last_frame = Instant::now();

        let mut sys = sysinfo::System::new();

        let mut do_tick_next = false;
        let mut do_tick_lqf_next = false;
        'mainLoop: loop {
            for event in event_pump.poll_iter() {
                if let Some(r) = &mut renderer {
                    r.imgui_sdl2.handle_event(&mut r.imgui, &event);
                    if r.imgui_sdl2.ignore_event(&event) { continue; }
                }

                let client_consumed_event = match &mut self.client {
                    Some(c) => c.on_event(&event),
                    None => false,
                };

                if !client_consumed_event {
                    match event {
                        Event::Quit {..} |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                            break 'mainLoop
                        },
                        Event::KeyDown { keycode: Some(Keycode::F11), .. } => {
                            self.settings.fullscreen = !self.settings.fullscreen;
                        },
                        Event::KeyDown { keycode: Some(Keycode::RShift | Keycode::LShift), .. } => {
                            shift_key = true;
                        },
                        Event::KeyUp { keycode: Some(Keycode::RShift | Keycode::LShift), .. } => {
                            shift_key = false;
                        },
                        Event::MouseWheel { y, .. } => {
                            if let Some(c) = &mut self.client {
                                if shift_key {
                                    let mut v = c.camera.scale + 0.1 * y as f64;
                                    if y > 0 {
                                        v = v.ceil();
                                    }else {
                                        v = v.floor();
                                    }

                                    v = v.clamp(1.0, 10.0);
                                    c.camera.scale = v;
                                }else{
                                    c.camera.scale = (c.camera.scale * (1.0 + 0.1 * y as f64)).clamp(0.01, 10.0);
                                }
                            }
                        },
                        Event::MouseButtonDown{mouse_btn: sdl2::mouse::MouseButton::Right, x, y, ..} => {
                            if let Some(w) = &mut self.world {
                                if let Some(ref r) = renderer {
                                    if let Some(ref c) = &mut self.client {
                                        let world_x = c.camera.x + (x as f64 - r.window.size().0 as f64 / 2.0) / c.camera.scale;
                                        let world_y = c.camera.y + (y as f64 - r.window.size().1 as f64 / 2.0) / c.camera.scale;
                                        let (chunk_x, chunk_y) = w.chunk_handler.pixel_to_chunk_pos(world_x as i64, world_y as i64);
                                        w.chunk_handler.force_update_chunk(chunk_x, chunk_y);
                                    }
                                }
                            }
                        },
                        Event::MouseMotion{xrel, yrel, mousestate , x, y, ..} => {
                            if mousestate.left() {
                                if let Some(c) = &mut self.client {
                                    // this doesn't do anything if game.client_entity_id exists
                                    //     since the renderer will snap the camera to the client entity
                                    c.camera.x -= (xrel as f64) / c.camera.scale;
                                    c.camera.y -= (yrel as f64) / c.camera.scale;
                                }
                            }else if mousestate.middle() {
                                if let Some(w) = &mut self.world {
                                    if let Some(ref c) = &mut self.client {
                                        if let Some(ref r) = renderer {
                                            let world_x = c.camera.x + (x as f64 - r.window.size().0 as f64 / 2.0) / c.camera.scale;
                                            let world_y = c.camera.y + (y as f64 - r.window.size().1 as f64 / 2.0) / c.camera.scale;

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
                            }
                        },
                        Event::Window{win_event: WindowEvent::Resized(w, h), ..} => {
                            GPUSubsystem::set_window_resolution(w as u16, h as u16);
                        },
                        _ => {}
                    }
                }
            }

            let now = std::time::Instant::now();

            if let Some(r) = &mut renderer {
                r.imgui_sdl2.prepare_frame(r.imgui.io_mut(), &r.window, &event_pump.mouse_state());
                let delta = now.saturating_duration_since(last_frame);
                let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
                last_frame = now;
                r.imgui.io_mut().delta_time = delta_s;
            }

            if let Some(r) = &mut renderer {
                let fs= r.window.fullscreen_state();

                let des_fs = match self.settings {
                    Settings {fullscreen, fullscreen_type, ..} if fullscreen && fullscreen_type == 0 => FullscreenType::Desktop,
                    Settings {fullscreen, fullscreen_type, ..} if fullscreen && fullscreen_type != 0 => FullscreenType::True,
                    _ => FullscreenType::Off,
                };

                if fs != des_fs {
                    println!("{:?}", des_fs);

                    if des_fs == FullscreenType::True {
                        r.window.set_fullscreen(FullscreenType::Off).unwrap();
                        r.window.maximize();
                    }else if des_fs == FullscreenType::Desktop {
                        r.window.restore();
                    }

                    r.window.set_fullscreen(des_fs).unwrap();

                    if des_fs == FullscreenType::Off {
                        r.window.restore();
                        GPUSubsystem::set_window_resolution(r.window.size().0 as u16, r.window.size().1 as u16);
                    }

                }
                
                if sdl2::hint::get_video_minimize_on_focus_loss() != self.settings.minimize_on_lost_focus {
                    sdl2::hint::set_video_minimize_on_focus_loss(self.settings.minimize_on_lost_focus);
                }

                let si_des = if self.settings.vsync { SwapInterval::VSync } else { SwapInterval::Immediate };
                if sdl.sdl_video.gl_get_swap_interval() != si_des {
                    sdl.sdl_video.gl_set_swap_interval(si_des).unwrap();
                }
            }

            // tick

            let mut can_tick = self.settings.tick;

            if let Some(r) = &renderer {
               let flags = r.window.window_flags();
               can_tick = can_tick && !(self.settings.pause_on_lost_focus && renderer.is_some() && !(flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32 == SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32));
            }

            if do_tick_next && can_tick {
                prev_tick_time = now;
                let st = Instant::now();
                self.tick();
                self.fps_counter.tick_times.rotate_left(1);
                self.fps_counter.tick_times[self.fps_counter.tick_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;
            }
            do_tick_next = can_tick && now.saturating_duration_since(prev_tick_time).as_nanos() > 1_000_000_000 / self.settings.tick_speed as u128; // intended is 30 ticks per second

            // tick liquidfun

            let mut can_tick = self.settings.tick_lqf;

            if let Some(r) = &renderer {
               let flags = r.window.window_flags();
               can_tick = can_tick && !(self.settings.pause_on_lost_focus && renderer.is_some() && !(flags & SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32 == SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32));
            }

            if do_tick_lqf_next && can_tick {
                prev_tick_lqf_time = now;
                if let Some(w) = &mut self.world {
                    let st = Instant::now();
                    w.tick_lqf(&self.settings);
                    self.fps_counter.tick_lqf_times.rotate_left(1);
                    self.fps_counter.tick_lqf_times[self.fps_counter.tick_lqf_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;
                    
                }
            }
            do_tick_lqf_next = can_tick && now.saturating_duration_since(prev_tick_lqf_time).as_nanos() > 1_000_000_000 / self.settings.tick_lqf_speed as u128; // intended is 60 ticks per second

            // render

            if let Some(r) = &mut renderer {
                profiling::scope!("rendering");

                let delta_time = Instant::now().saturating_duration_since(counter_last_frame).as_secs_f64();

                r.render(sdl, self, delta_time);

                self.frame_count += 1;
                self.fps_counter.frames += 1;
                if now.saturating_duration_since(self.fps_counter.last_update).as_millis() >= 1000 {
                    self.fps_counter.display_value = self.fps_counter.frames;
                    self.fps_counter.frames = 0;
                    self.fps_counter.last_update = now;
                    let set = r.window.set_title(format!("FallingSandRust ({} FPS)", self.fps_counter.display_value).as_str());
                    if set.is_err() {
                        eprintln!("Failed to set window title.");
                    }
                    
                    sys.refresh_process(std::process::id() as Pid);
                    if let Some(pc) = sys.process(std::process::id() as Pid) {
                        self.process_stats.cpu_usage = Some(pc.cpu_usage() / sys.processors().len() as f32);
                        self.process_stats.memory = Some(pc.memory());
                    }
                }
            }

            let time_nano = Instant::now().saturating_duration_since(counter_last_frame).as_nanos();
            self.fps_counter.frame_times.rotate_left(1);
            self.fps_counter.frame_times[self.fps_counter.frame_times.len() - 1] = time_nano as f32;

            profiling::finish_frame!();
            // sleep
            if !do_tick_next {
                ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
            }
            counter_last_frame = Instant::now();
        }

        println!("Closing...");
    }

    #[profiling::function]
    fn tick(&mut self){
        self.tick_time += 1;

        if let Some(w) = &mut self.world {
            w.tick(self.tick_time, &self.settings);
            if let Some(cw) = &mut self.client {
                cw.tick(w);
            }
        }
    }

}