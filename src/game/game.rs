
use crate::game::common::Settings;
use crate::game::common::world::World;

use std::time::Instant;

use super::client::Client;
use super::common::FileHelper;
use super::common::world::Chunk;


pub struct Game<C: Chunk> {
    pub world: Option<World<C>>,
    pub tick_time: u32,
    pub frame_count: u32,
    pub fps_counter: FPSCounter,
    pub process_stats: ProcessStats,
    pub settings: Settings,
    pub file_helper: FileHelper,
    pub client: Option<Client>,
}

pub struct ProcessStats {
    pub cpu_usage: Option<f32>,
    pub memory: Option<u64>,
}

pub struct FPSCounter {
    pub frames: u16,
    pub last_update: Instant,
    pub display_value: u16,
    pub ticks: u16,
    pub tick_display_value: u16,
    pub frame_times: [f32; 200],
    pub tick_times: [f32; 200],
    pub tick_lqf_times: [f32; 200],
}

impl<'a, 'b, C: Chunk> Game<C> {
    #[profiling::function]
    pub fn new(file_helper: FileHelper) -> Self {
        Game {
            world: Some(World::create()),
            tick_time: 0,
            frame_count: 0,
            fps_counter: FPSCounter {
                frames: 0, 
                last_update: Instant::now(), 
                display_value: 0,
                ticks: 0,
                tick_display_value: 0,
                frame_times: [0.0; 200],
                tick_times: [0.0; 200],
                tick_lqf_times: [0.0; 200],
            },
            process_stats: ProcessStats {
                cpu_usage: None,
                memory: None,
            },
            settings: Settings::default(),
            file_helper,
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

}