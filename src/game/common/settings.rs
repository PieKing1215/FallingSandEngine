pub struct Settings {
    pub debug: bool,

    // rendering
    pub draw_chunk_state_overlay: bool,
    pub draw_chunk_state_overlay_alpha: f32,
    pub draw_chunk_dirty_rects: bool,
    pub draw_chunk_grid: bool,
    pub draw_chunk_collision: usize,
    pub draw_origin: bool,
    pub draw_load_zones: bool,
    pub cull_chunks: bool,
    pub lqf_dbg_draw: bool,
    pub lqf_dbg_draw_shape: bool,
    pub lqf_dbg_draw_joint: bool,
    pub lqf_dbg_draw_aabb: bool,
    pub lqf_dbg_draw_pair: bool,
    pub lqf_dbg_draw_center_of_mass: bool,
    pub lqf_dbg_draw_particle: bool,

    // display
    pub fullscreen: bool,
    pub fullscreen_type: usize,
    pub vsync: bool,
    pub minimize_on_lost_focus: bool,

    // simulation
    pub tick: bool,
    pub tick_speed: u16,
    pub tick_lqf: bool,
    pub tick_lqf_speed: u16,
    pub tick_lqf_timestep: f32,
    pub load_chunks: bool,
    pub simulate_chunks: bool,
    pub simulate_particles: bool,
    pub pause_on_lost_focus: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            debug: false,
            draw_chunk_state_overlay: false,
            draw_chunk_state_overlay_alpha: 0.5,
            draw_chunk_dirty_rects: false,
            draw_chunk_grid: false,
            draw_chunk_collision: 0,
            draw_origin: true,
            draw_load_zones: false,
            cull_chunks: true,
            lqf_dbg_draw: false,
            lqf_dbg_draw_shape: true,
            lqf_dbg_draw_joint: true,
            lqf_dbg_draw_aabb: false,
            lqf_dbg_draw_pair: true,
            lqf_dbg_draw_center_of_mass: true,
            lqf_dbg_draw_particle: false,

            fullscreen: false,
            fullscreen_type: 0,
            vsync: false,
            minimize_on_lost_focus: false,

            tick: true,
            tick_speed: 30,
            tick_lqf: true,
            tick_lqf_speed: 60,
            tick_lqf_timestep: 1.0 / 45.0,
            load_chunks: true,
            simulate_chunks: true,
            simulate_particles: true,
            pause_on_lost_focus: false,
        }
    }
}
