use std::{cell::RefCell, fs};

use imgui::{WindowFlags};
use imgui_glow_renderer::{AutoRenderer, versions::GlVersion};
use imgui_sdl2_support::SdlPlatform;
use sdl2::{
    pixels::Color,
    ttf::{Font, Sdl2TtfContext},
    video::{Window, GLProfile},
    VideoSubsystem,
};
use sdl_gpu::{shaders::Shader, GPUImage, GPURect, GPUSubsystem, GPUTarget};

use super::TransformStack;
use crate::game::{
    client::world::{ClientChunk, WorldRenderer},
    common::FileHelper,
    Game,
};

pub struct Renderer<'ttf> {
    pub fonts: Option<Fonts<'ttf>>,
    pub shaders: Shaders,
    pub target: RefCell<GPUTarget>,
    pub window: Window,
    pub imgui: imgui::Context,
    pub imgui_sdl2: SdlPlatform,
    pub imgui_renderer: AutoRenderer,
    pub world_renderer: WorldRenderer,
    pub version_info_cache_1: Option<(u32, u32, GPUImage)>,
    pub version_info_cache_2: Option<(u32, u32, GPUImage)>,
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
    #[profiling::function]
    pub fn init_sdl() -> Result<Sdl2Context, String> {
        let sdl = sdl2::init()?;
        let sdl_video = sdl.video()?;
        let sdl_ttf = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();

        Ok(Sdl2Context { sdl, sdl_video, sdl_ttf })
    }

    #[profiling::function]
    pub fn create(sdl: &Sdl2Context, file_helper: &FileHelper) -> Result<Self, String> {
        let gl_attr = sdl.sdl_video.gl_attr();
        gl_attr.set_context_version(4, 1);
        gl_attr.set_context_profile(GLProfile::Core);

        let window = {
            profiling::scope!("window");

            sdl
                .sdl_video
                .window("FallingSandRust", 1200, 800)
                .opengl() // allow getting opengl context
                .resizable()
                .build()
                .unwrap()
        };

        sdl_gpu::GPUSubsystem::set_init_window(window.id());
        let target = {
            profiling::scope!("GPUSubsystem::init");
            sdl_gpu::GPUSubsystem::init(window.size().0 as u16, window.size().1 as u16, 0)
        };
        unsafe {
            let ctx: sdl2::sys::SDL_GLContext = (*target.raw.context).context;
            sdl2::sys::SDL_GL_MakeCurrent(window.raw(), ctx);
        }

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let gl = unsafe {
            glow::Context::from_loader_function(|s| sdl.sdl_video.gl_get_proc_address(s).cast())
        };
        let v = GlVersion::read(&gl);
        log::info!("glversion = {} {} {}", v.major, v.minor, v.is_gles);
        let imgui_renderer = AutoRenderer::initialize(gl, &mut imgui).unwrap();
        let imgui_sdl2 = SdlPlatform::init(&mut imgui);

        let shaders = Shaders {
            liquid_shader: Shader::load_shader_program(
                fs::read_to_string(file_helper.asset_path("data/shaders/common.vert"))
                    .map_err(|e| e.to_string())?
                    .as_str(),
                fs::read_to_string(file_helper.asset_path("data/shaders/liquid.frag"))
                    .map_err(|e| e.to_string())?
                    .as_str(),
            )?,
        };

        Ok(Renderer {
            fonts: None,
            shaders,
            target: RefCell::new(target),
            window,
            imgui,
            imgui_sdl2,
            imgui_renderer,
            world_renderer: WorldRenderer::new(),
            version_info_cache_1: None,
            version_info_cache_2: None,
        })
    }

    #[profiling::function]
    pub fn render(
        &mut self,
        sdl: &Sdl2Context,
        game: &mut Game<ClientChunk>,
        delta_time: f64,
        partial_ticks: f64,
    ) {
        self.target.borrow_mut().clear();

        self.render_internal(sdl, game, delta_time, partial_ticks);

        let target = &mut self.target.borrow_mut();

        {
            profiling::scope!("version info");

            let (w, h, img) = self.version_info_cache_1.get_or_insert_with(|| {
                let surf = self
                    .fonts
                    .as_ref()
                    .unwrap()
                    .pixel_operator
                    .render("Development Build")
                    .solid(Color::RGB(0xff, 0xff, 0xff)).unwrap();
                (surf.width(), surf.height(), GPUImage::from_surface(&surf))
            });

            img.blit_rect(
                None,
                target,
                Some(GPURect::new(
                    4.0,
                    self.window.size().1 as f32 - 4.0 - 14.0 * 2.0,
                    *w as f32,
                    *h as f32,
                )),
            );

            let (w, h, img) = self.version_info_cache_2.get_or_insert_with(|| {
                let surf = self
                    .fonts
                    .as_ref()
                    .unwrap()
                    .pixel_operator
                    .render(format!("{} ({})", env!("BUILD_DATETIME"), env!("GIT_HASH")).as_str())
                    .solid(Color::RGB(0xff, 0xff, 0xff)).unwrap();
                    (surf.width(), surf.height(), GPUImage::from_surface(&surf))
            });

            img.blit_rect(
                None,
                target,
                Some(GPURect::new(
                    4.0,
                    self.window.size().1 as f32 - 4.0 - 14.0,
                    *w as f32,
                    *h as f32,
                )),
            );
        }

        {
            profiling::scope!("imgui");
            let ui = self.imgui.new_frame();

            // ui.show_demo_window(&mut true);

            if game.settings.debug {
                game.settings.imgui(ui);
            }

            game.client
                .as_mut()
                .expect("Missing client in Renderer::render ??")
                .main_menu
                .render(ui, &game.file_helper);

            ui.window("Stats")
                .size([300.0, 300.0], imgui::Condition::FirstUseEver)
                .position_pivot([1.0, 1.0])
                .position(
                    [self.window.size().0 as f32, self.window.size().1 as f32],
                    imgui::Condition::Always,
                )
                .flags(
                    WindowFlags::ALWAYS_AUTO_RESIZE
                        | WindowFlags::NO_DECORATION
                        | WindowFlags::NO_MOUSE_INPUTS
                        | WindowFlags::NO_FOCUS_ON_APPEARING
                        | WindowFlags::NO_NAV,
                )
                .bg_alpha(0.25)
                .resizable(false)
                .build(|| {
                    ui.text(match game.process_stats.cpu_usage {
                        Some(c) => format!("CPU: {:.0}%", c),
                        None => "CPU: n/a".to_string(),
                    });
                    ui.same_line();
                    ui.text(match game.process_stats.memory {
                        Some(m) => format!(" mem: {:.1} MB", m as f32 / 1000.0),
                        None => " mem: n/a".to_string(),
                    });

                    let nums: Vec<&f32> = game
                        .fps_counter
                        .frame_times
                        .iter()
                        .filter(|n| **n != 0.0)
                        .collect();
                    let avg_mspf: f32 =
                        nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                    ui.plot_lines("", &game.fps_counter.frame_times)
                        .graph_size([200.0, 50.0])
                        .scale_min(0.0)
                        .scale_max(50_000_000.0)
                        .overlay_text(
                            format!("mspf: {:.2} fps: {:.0}", avg_mspf, ui.io().framerate),
                        )
                        .build();

                    let nums: Vec<&f32> = game
                        .fps_counter
                        .tick_times
                        .iter()
                        .filter(|n| **n != 0.0)
                        .collect();
                    let avg_mspt: f32 =
                        nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                    ui.plot_histogram("", &game.fps_counter.tick_times)
                        .graph_size([200.0, 50.0])
                        .scale_min(0.0)
                        .scale_max(100_000_000.0)
                        .overlay_text(format!("tick mspt: {:.2}", avg_mspt))
                        .build();

                    let nums: Vec<&f32> = game
                        .fps_counter
                        .tick_lqf_times
                        .iter()
                        .filter(|n| **n != 0.0)
                        .collect();
                    let avg_msptlqf: f32 =
                        nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                    ui.plot_histogram("", &game.fps_counter.tick_lqf_times)
                        .graph_size([200.0, 50.0])
                        .scale_min(0.0)
                        .scale_max(100_000_000.0)
                        .overlay_text(format!("phys mspt: {:.2}", avg_msptlqf))
                        .build();
                });

            let draw_data = {
                profiling::scope!("prepare_render");
                self.imgui.render()
            };
            {
                profiling::scope!("render");
                self.imgui_renderer.render(draw_data).unwrap();
            }
        }

        target.flip();
    }

    #[profiling::function]
    fn render_internal(
        &mut self,
        sdl: &Sdl2Context,
        game: &mut Game<ClientChunk>,
        delta_time: f64,
        partial_ticks: f64,
    ) {
        let target = &mut self.target.borrow_mut();

        {
            profiling::scope!("test stuff");
            target.rectangle2(
                GPURect::new(
                    40.0 + ((game.tick_time as f32 / 5.0).sin() * 20.0),
                    30.0 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0),
                    15.0,
                    15.0,
                ),
                Color::RGBA(255, 0, 0, 255),
            );

            GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);
            for i in (0..10000).step_by(15) {
                let thru = (i as f32 / 10000.0 * 255.0) as u8;
                let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
                let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;

                let rect = GPURect::new(
                    75.0 + (i as f32 % 1000.0)
                        + (((game.frame_count as f32 / 2.0 + (i as i32 / 2) as f32
                            - timeshift as f32)
                            / 100.0)
                            .sin()
                            * 50.0),
                    (i as f32 / 1000.0) * 100.0
                        + (((game.frame_count as f32 / 2.0 + (i as i32 / 2) as f32
                            - timeshift as f32)
                            / 100.0)
                            .cos()
                            * 50.0),
                    20.0,
                    20.0,
                );
                target.rectangle_filled2(rect, Color::RGBA(0, thru, 255 - thru, thru2));
            }
        }

        if let Some(w) = &mut game.world {
            self.world_renderer.render(
                w,
                target,
                &mut TransformStack::new(),
                delta_time,
                sdl,
                self.fonts.as_ref().unwrap(),
                &game.settings,
                &self.shaders,
                &mut game.client,
                partial_ticks,
            );
        }
    }
}
