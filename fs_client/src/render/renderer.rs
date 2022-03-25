use std::fs;

use fs_common::game::{
    common::{world::material::Color, FileHelper, Rect},
    GameData,
};
use glium::{Display, DrawParameters, PolygonMode, Blend};
use glium_glyph::{glyph_brush::{rusttype::Font, Section}, GlyphBrush};
use glutin::{dpi::LogicalSize, event_loop::EventLoop};
use imgui::WindowFlags;
use imgui_winit_support::{WinitPlatform, HiDpiMode};

use crate::{
    render::imgui::DebugUI,
    world::{ClientChunk, WorldRenderer},
    Client,
};

use super::{drawing::RenderTarget, shaders::Shaders};

pub static mut BUILD_DATETIME: Option<&str> = None;
pub static mut GIT_HASH: Option<&str> = None;

pub struct Renderer<'a> {
    // pub fonts: Fonts,
    pub glyph_brush: GlyphBrush<'a, 'a>,
    pub shaders: Shaders,
    pub display: Display,
    pub world_renderer: WorldRenderer,
    pub imgui: imgui::Context,
    pub imgui_platform: WinitPlatform,
    pub imgui_renderer: imgui_glium_renderer::Renderer,
    // pub version_info_cache_1: Option<(u32, u32, GPUImage)>,
    // pub version_info_cache_2: Option<(u32, u32, GPUImage)>,
}

pub struct Fonts {
    // pub pixel_operator: Font<'ttf, 'static>,
}

impl<'a> Renderer<'a> {
    #[profiling::function]
    pub fn create(event_loop: &EventLoop<()>, file_helper: &FileHelper) -> Result<Self, String> {
        let wb = glutin::window::WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1200_i16, 800_i16))
            .with_title("FallingSandRust");
        let cb = glutin::ContextBuilder::new();
        let display = glium::Display::new(wb, cb, event_loop).unwrap();

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let mut imgui_platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();
    
            imgui_platform.attach_window(imgui.io_mut(), window, HiDpiMode::Default);
        }

        log::info!("glversion = {:?}", display.get_opengl_version());
        let imgui_renderer = imgui_glium_renderer::Renderer::init(&mut imgui, &display).unwrap();
        // let imgui_sdl2 = SdlPlatform::init(&mut imgui);

        // let shaders = Shaders {
        //     liquid_shader: Shader::load_shader_program(
        //         fs::read_to_string(file_helper.asset_path("data/shaders/common.vert"))
        //             .map_err(|e| e.to_string())?
        //             .as_str(),
        //         fs::read_to_string(file_helper.asset_path("data/shaders/liquid.frag"))
        //             .map_err(|e| e.to_string())?
        //             .as_str(),
        //     )?,
        // };

        let shaders = Shaders::new(&display, file_helper);

        let pixel_operator = fs::read(file_helper.asset_path("font/pixel_operator/PixelOperator.ttf")).unwrap();
        let fonts = vec![Font::from_bytes(pixel_operator).unwrap()];

        let glyph_brush = GlyphBrush::new(&display, fonts);

        Ok(Renderer {
            glyph_brush,
            shaders,
            display,
            world_renderer: WorldRenderer::new(),
            imgui,
            imgui_platform,
            imgui_renderer,
            // version_info_cache_1: None,
            // version_info_cache_2: None,
        })
    }

    #[profiling::function]
    pub fn render(
        &mut self,
        game: &mut GameData<ClientChunk>,
        client: &mut Client,
        delta_time: f64,
        partial_ticks: f64,
    ) {
        let mut target = RenderTarget::new(&mut self.display, &self.shaders, &mut self.glyph_brush);
        target.clear(Color::BLACK);

        Self::render_internal(&mut self.world_renderer, &mut target, game, client, delta_time, partial_ticks);

        {
            profiling::scope!("version info");

            target.queue_text(Section {
                text: "Development Build",
                screen_position: (4.0, target.height() as f32 - 40.0),
                bounds: (150.0, 20.0),
                color: Color::WHITE.into(),
                ..Section::default()
            });
            target.queue_text(Section {
                text: format!(
                        "{} ({})",
                        unsafe { BUILD_DATETIME }.unwrap_or("???"),
                        unsafe { GIT_HASH }.unwrap_or("???")
                    )
                    .as_str(),
                screen_position: (4.0, target.height() as f32 - 20.0),
                bounds: (200.0, 20.0),
                color: Color::WHITE.into(),
                ..Section::default()
            });
            target.draw_queued_text();
        }

        {
            profiling::scope!("imgui");
            let ui = self.imgui.new_frame();

            // ui.show_demo_window(&mut true);

            if game.settings.debug {
                // TODO: reimplement vsync for glutin
                // let last_vsync = game.settings.vsync;
                // let last_minimize_on_lost_focus = game.settings.minimize_on_lost_focus;

                game.settings.debug_ui(ui);

                // TODO: this should be somewhere better
                // maybe clone the Settings before each frame and at the end compare it?

                // if game.settings.vsync != last_vsync {
                //     let si_des = if game.settings.vsync {
                //         SwapInterval::VSync
                //     } else {
                //         SwapInterval::Immediate
                //     };

                //     self.sdl.as_ref().unwrap().sdl_video.gl_set_swap_interval(si_des).unwrap();
                // }

                // if last_minimize_on_lost_focus != game.settings.minimize_on_lost_focus {
                //     sdl2::hint::set_video_minimize_on_focus_loss(
                //         game.settings.minimize_on_lost_focus,
                //     );
                // }
            }

            client.main_menu.render(ui, &game.file_helper);

            ui.window("Stats")
                .size([300.0, 300.0], imgui::Condition::FirstUseEver)
                .position_pivot([1.0, 1.0])
                .position(
                    [target.width() as f32, target.height() as f32],
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
                        .overlay_text(format!(
                            "mspf: {:.2} fps: {:.0}/{:.0}",
                            avg_mspf,
                            ui.io().framerate,
                            1_000_000_000.0
                                / game
                                    .fps_counter
                                    .frame_times
                                    .iter()
                                    .copied()
                                    .reduce(f32::max)
                                    .unwrap()
                        ))
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
                        .tick_physics_times
                        .iter()
                        .filter(|n| **n != 0.0)
                        .collect();
                    let avg_mspt_physics: f32 =
                        nums.iter().map(|f| *f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                    ui.plot_histogram("", &game.fps_counter.tick_physics_times)
                        .graph_size([200.0, 50.0])
                        .scale_min(0.0)
                        .scale_max(100_000_000.0)
                        .overlay_text(format!("phys mspt: {:.2}", avg_mspt_physics))
                        .build();
                });

            let draw_data = {
                profiling::scope!("prepare_render");
                self.imgui_platform.prepare_render(ui, self.display.gl_window().window());
                self.imgui.render()
            };
            {
                profiling::scope!("render");
                self.imgui_renderer.render(&mut target.frame, draw_data).unwrap();
            }
        }

        target.finish().unwrap();
    }

    #[profiling::function]
    fn render_internal(
        world_renderer: &mut WorldRenderer,
        target: &mut RenderTarget,
        game: &mut GameData<ClientChunk>,
        client: &mut Client,
        delta_time: f64,
        partial_ticks: f64,
    ) {

        target.base_transform.push();
        target.base_transform.translate(-1.0, 1.0);
        target.base_transform.scale(2.0 / target.width() as f64, -2.0 / target.height() as f64);

        {
            profiling::scope!("test stuff");
            target.rectangle(
                Rect::new_wh(
                    40.0 + ((game.tick_time as f32 / 5.0).sin() * 20.0),
                    30.0 + ((game.tick_time as f32 / 5.0).cos().abs() * -10.0),
                    15.0,
                    15.0,
                ),
                Color::rgb(255, 0, 0),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    ..Default::default()
                }
            );

            let rects = (0..10000).step_by(15).map(|i|{
                let thru = (i as f32 / 10000.0 * 255.0) as u8;
                let thru2 = (((i % 1000) as f32 / 1000.0) * 255.0) as u8;
                let timeshift = ((1.0 - ((i % 1000) as f32 / 1000.0)).powi(8) * 200.0) as i32;

                let color = Color::rgba(0, thru, 255 - thru, thru2);

                (Rect::new_wh(
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
                ), color)
            }).collect::<Vec<_>>();
            target.rectangles_colored(
                &rects,
                DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                }
            );
        }

        if let Some(w) = &mut game.world {
            // let pixel_operator2 = self.sdl.as_ref().unwrap()
            //     .sdl_ttf
            //     .load_font(
            //         game.file_helper.asset_path("font/pixel_operator/PixelOperator.ttf"),
            //         16,
            //     )
            //     .unwrap();
            // let f = Fonts { pixel_operator: pixel_operator2 };

            world_renderer.render(
                w,
                target,
                delta_time,
                &game.settings,
                client,
                partial_ticks,
            );
        }

        target.base_transform.pop();
    }
}
