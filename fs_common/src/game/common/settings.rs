use super::{registry::RegistryID, world::gen::structure::set::StructureSet};

pub struct Settings {
    pub debug: bool,

    // rendering
    pub draw_chunk_state_overlay: bool,
    pub draw_chunk_state_overlay_alpha: f32,
    pub draw_chunk_dirty_rects: bool,
    pub draw_chunk_grid: bool,
    pub draw_chunk_collision: ChunkCollisionOverlay,
    pub draw_origin: bool,
    pub draw_load_zones: bool,
    pub draw_structure_bounds: bool,
    pub draw_structure_set: Option<RegistryID<StructureSet>>,
    pub draw_lighting: bool,
    pub lighting_smooth: bool,
    pub lighting_dithering: bool,
    pub lighting_overlay: bool,
    pub lighting_linear_blend: bool,
    pub cull_chunks: bool,
    pub physics_dbg_draw: bool,
    pub physics_dbg_draw_shape: bool,
    pub physics_dbg_draw_joint: bool,
    pub physics_dbg_draw_aabb: bool,
    pub physics_dbg_draw_pair: bool,
    pub physics_dbg_draw_center_of_mass: bool,
    pub physics_dbg_draw_particle: bool,

    // display
    pub fullscreen: bool,
    pub fullscreen_type: usize,
    pub vsync: bool,
    pub minimize_on_lost_focus: bool,

    // simulation
    pub tick: bool,
    pub tick_speed: u16,
    pub tick_physics: bool,
    pub tick_physics_speed: u16,
    pub tick_physics_timestep: f32,
    pub load_chunks: bool,
    pub simulate_chunks: bool,
    pub simulate_particles: bool,
    pub pause_on_lost_focus: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkCollisionOverlay {
    None,
    MarchingSquares,
    RamerDouglasPeucker,
    Earcutr,
}

impl ChunkCollisionOverlay {
    pub fn values() -> &'static [Self] {
        &[
            Self::None,
            Self::MarchingSquares,
            Self::RamerDouglasPeucker,
            Self::Earcutr,
        ]
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            debug: false,
            draw_chunk_state_overlay: false,
            draw_chunk_state_overlay_alpha: 0.5,
            draw_chunk_dirty_rects: false,
            draw_chunk_grid: false,
            draw_chunk_collision: ChunkCollisionOverlay::None,
            draw_origin: true,
            draw_load_zones: false,
            draw_structure_bounds: false,
            draw_structure_set: None,

            draw_lighting: true,
            lighting_smooth: true,
            lighting_dithering: true,
            lighting_overlay: false,
            lighting_linear_blend: true,

            cull_chunks: true,
            physics_dbg_draw: false,
            physics_dbg_draw_shape: true,
            physics_dbg_draw_joint: true,
            physics_dbg_draw_aabb: false,
            physics_dbg_draw_pair: true,
            physics_dbg_draw_center_of_mass: true,
            physics_dbg_draw_particle: false,

            fullscreen: false,
            fullscreen_type: 0,
            vsync: false,
            minimize_on_lost_focus: false,

            tick: true,
            tick_speed: 30,
            tick_physics: true,
            tick_physics_speed: 60,
            tick_physics_timestep: 1.0 / 45.0,
            load_chunks: true,
            simulate_chunks: true,
            simulate_particles: true,
            pause_on_lost_focus: false,
        }
    }
}
