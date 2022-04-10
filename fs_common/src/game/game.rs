use crate::game::common::world::World;
use crate::game::common::Settings;

use std::time::Instant;

use super::common::world::material::placer::MaterialPlacerRegistry;
use super::common::world::material::{placer, MaterialRegistry};
use super::common::world::{material, Chunk};
use super::common::FileHelper;

pub struct Registries {
    pub materials: MaterialRegistry,
    pub material_placers: MaterialPlacerRegistry,
}

impl Default for Registries {
    fn default() -> Self {
        Self {
            materials: material::init_material_types(),
            material_placers: placer::init_material_placers(),
        }
    }
}

pub struct GameData<C: Chunk> {
    pub world: Option<World<C>>,
    pub tick_time: u32,
    pub frame_count: u32,
    pub fps_counter: FPSCounter,
    pub process_stats: ProcessStats,
    pub settings: Settings,
    pub file_helper: FileHelper,
    pub registries: Registries,
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
    pub tick_physics_times: [f32; 200],
}

impl<C: Chunk> GameData<C> {
    #[profiling::function]
    pub fn new(file_helper: FileHelper) -> Self {
        GameData {
            world: Some(World::create(None)),
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
                tick_physics_times: [0.0; 200],
            },
            process_stats: ProcessStats { cpu_usage: None, memory: None },
            settings: Settings::default(),
            file_helper,
            registries: Registries::default(),
        }
    }
}
