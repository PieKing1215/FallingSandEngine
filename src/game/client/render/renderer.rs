
use std::cell::RefCell;

use imgui::{WindowFlags, im_str};
use sdl2::{VideoSubsystem, pixels::Color, ttf::{Font, Sdl2TtfContext}, video::Window};
use sdl_gpu::{GPURect, GPUSubsystem, GPUTarget, shaders::Shader};

use super::TransformStack;
use crate::game::{Game, client::world::{ClientChunk, WorldRenderer}};

pub struct Renderer<'ttf> {
    pub fonts: Option<Fonts<'ttf>>,
    pub shaders: Shaders,
    pub target: RefCell<GPUTarget>,
    pub window: Window,
    pub imgui: imgui::Context,
    pub imgui_sdl2: imgui_sdl2::ImguiSdl2,
    pub imgui_renderer: imgui_opengl_renderer::Renderer,
    pub world_renderer: WorldRenderer,
}

pub struct Fonts<'ttf> {
    pub pixel_operator: Font<'ttf, 'static>,
}

pub struct Shaders {
    pub liquid_shader: Shader,
}

pub struct Sdl2Context {
    pub sdl: sdl2::Sdl,
    pub sdl_video: VideoSubsystem,
    pub sdl_ttf: Sdl2TtfContext,
}

impl<'a> Renderer<'a> {

    pub fn init_sdl() -> Result<Sdl2Context, String> {
        let sdl = sdl2::init()?;
        let sdl_video = sdl.video()?;
        let sdl_ttf = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();

        Ok(Sdl2Context {
            sdl, 
            sdl_video, 
            sdl_ttf,
        })
    }

    pub fn create(sdl: &Sdl2Context) -> Result<Self, String> {
        
        let window = sdl.sdl_video.window("FallingSandRust", 1200, 800)
            .opengl() // allow getting opengl context
            .resizable()
            .build()
            .unwrap();
    
        sdl_gpu::GPUSubsystem::set_init_window(window.id());
        let target = sdl_gpu::GPUSubsystem::init(window.size().0 as u16, window.size().1 as u16, 0);
        unsafe {
            let ctx: sdl2::sys::SDL_GLContext = (*target.raw.context).context as sdl2::sys::SDL_GLContext;
            sdl2::sys::SDL_GL_MakeCurrent(window.raw(), ctx);
        }

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
      
        let imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui, &window);
        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, |s| sdl.sdl_video.gl_get_proc_address(s) as _);

        let shaders = Shaders {
            liquid_shader: Shader::load_shader_program(
                include_str!("../../../../assets/data/shaders/common.vert"), 
                include_str!("../../../../assets/data/shaders/liquid.frag"))?,
        };

        return Ok(Renderer {
            fonts: None,
            shaders,
            target: RefCell::new(target),
            window,
            imgui,
            imgui_sdl2,
            imgui_renderer: renderer,
            world_renderer: WorldRenderer::new(),
        });
    }

    #[profiling::function]
    pub fn render(&mut self, sdl: &Sdl2Context, game: &mut Game<ClientChunk>, delta_time: f64){

        self.target.borrow_mut().clear();

        self.render_internal(sdl, game, delta_time);

        let target = &mut self.target.borrow_mut();

        {
            profiling::scope!("imgui");
            let ui = self.imgui.frame();

            // ui.show_demo_window(&mut true);
            
            if game.settings.debug {
                game.settings.imgui(&ui);
            }

            imgui::Window::new(im_str!("Stats"))
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .position_pivot([1.0, 1.0])
            .position([self.window.size().0 as f32, self.window.size().1 as f32], imgui::Condition::Always)
            .flags(WindowFlags::ALWAYS_AUTO_RESIZE | WindowFlags::NO_DECORATION | WindowFlags::NO_MOUSE_INPUTS | WindowFlags::NO_FOCUS_ON_APPEARING | WindowFlags::NO_NAV)
            .bg_alpha(0.25)
            .resizable(false)
            .build(&ui, || {

                ui.text(match game.process_stats.cpu_usage {
                    Some(c) => format!("CPU: {:.0}%", c),
                    None => format!("CPU: n/a"),
                });
                ui.same_line(0.0);
                ui.text(match game.process_stats.memory {
                    Some(m) => format!(" mem: {:.1} MB", m as f32 / 1000.0),
                    None => format!(" mem: n/a"),
                });

                let nums: Vec<f32> = game.fps_counter.frame_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                ui.plot_lines(im_str!(""), &game.fps_counter.frame_times)
                    .graph_size([200.0, 50.0])
                    .scale_min(0.0)
                    .scale_max(50_000_000.0)
                    .overlay_text(im_str!("mspf: {:.2} fps: {:.0}", avg_mspt, ui.io().framerate).as_ref())
                    .build();

                let nums: Vec<f32> = game.fps_counter.tick_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                ui.plot_histogram(im_str!(""), &game.fps_counter.tick_times)
                    .graph_size([200.0, 50.0])
                    .scale_min(0.0)
                    .scale_max(100_000_000.0)
                    .overlay_text(im_str!("tick mspt: {:.2}", avg_mspt).as_ref())
                    .build();
                
                    
                let nums: Vec<f32> = game.fps_counter.tick_lqf_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                ui.plot_histogram(im_str!(""), &game.fps_counter.tick_lqf_times)
                    .graph_size([200.0, 50.0])
                    .scale_min(0.0)
                    .scale_max(100_000_000.0)
                    .overlay_text(im_str!("phys mspt: {:.2}", avg_mspt).as_ref())
                    .build();
            });

            {
                profiling::scope!("prepare_render");
                self.imgui_sdl2.prepare_render(&ui, &self.window);
            }
            {
                profiling::scope!("render");
                self.imgui_renderer.render(ui);
            }
        }

        target.flip();
    }

    #[profiling::function]
    fn render_internal(&mut self, sdl: &Sdl2Context, game: &mut Game<ClientChunk>, delta_time: f64){
        let target = &mut self.target.borrow_mut();
        
        target.rectangle2(GPURect::new(40.0 + ((game.tick_time as f32 / 5.0).sin() * 20.0), 
        30.0 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0), 
        15.0, 15.0), Color::RGBA(255, 0, 0, 255));

        GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);
        for i in (0..10000).step_by(15) {
            let thru = (i as f32 / 10000.0 * 255.0) as u8;
            let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
            let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;

            let rect = GPURect::new(75.0 + (i as f32 % 1000.0) + (((game.frame_count as i32/2 + (i as i32 / 2) - timeshift) as f32 / 100.0).sin() * 50.0), (i as f32 / 1000.0)*100.0 + (((game.frame_count as i32/2 + (i as i32 / 2) - timeshift) as f32 / 100.0).cos() * 50.0), 20.0, 20.0);
            target.rectangle_filled2(rect, Color::RGBA(0, thru, 255-thru, thru2));
        }

        if let Some(w) = &mut game.world {
            self.world_renderer.render(w, target, &mut TransformStack::new(), delta_time, sdl, &self.fonts.as_ref().unwrap(), &game.settings, &self.shaders, &mut game.client);
        }
        
    }
}