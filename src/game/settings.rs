use imgui::{ComboBox, Slider, SliderFlags, TreeNode, WindowFlags, im_str};

pub struct Settings {
    // rendering
    pub draw_chunk_state_overlay: bool,
    pub draw_chunk_state_overlay_alpha: f32,
    pub draw_chunk_dirty_rects: bool,
    pub draw_chunk_grid: bool,
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
    pub load_chunks: bool,
    pub pause_on_lost_focus: bool,
}

impl Settings {
    #[profiling::function]
    pub fn imgui(&mut self, ui: &imgui::Ui){
        imgui::Window::new(im_str!("Debug Menu"))
        .size([300.0, 600.0], imgui::Condition::FirstUseEver)
        .flags(WindowFlags::ALWAYS_AUTO_RESIZE)
        .resizable(false)
        .build(&ui, || {
            TreeNode::new(im_str!("rendering")).label(im_str!("rendering")).build(ui, || {
                // TreeNode::new(im_str!("chunk_overlay")).label(im_str!("chunk overlay")).build(ui, || {
                    ui.checkbox(im_str!("draw_chunk_state_overlay"), &mut self.draw_chunk_state_overlay);
                    if self.draw_chunk_state_overlay {
                        ui.indent();
                        ui.set_next_item_width(80.0);
                        Slider::new(im_str!("alpha"))
                            .range(0.1..=1.0)
                            .display_format(im_str!("%.1f"))
                            .flags(SliderFlags::ALWAYS_CLAMP)
                            .build(ui, &mut self.draw_chunk_state_overlay_alpha);
                        ui.unindent();
                    }
                    ui.checkbox(im_str!("draw_chunk_dirty_rects"), &mut self.draw_chunk_dirty_rects);
                    ui.checkbox(im_str!("draw_chunk_grid"), &mut self.draw_chunk_grid);
                    ui.checkbox(im_str!("draw_origin"), &mut self.draw_origin);
                    ui.checkbox(im_str!("draw_load_zones"), &mut self.draw_load_zones);
                    ui.checkbox(im_str!("cull_chunks"), &mut self.cull_chunks);

                    ui.checkbox(im_str!("lqf_dbg_draw"), &mut self.lqf_dbg_draw);
                    ui.indent();
                    ui.checkbox(im_str!("lqf_dbg_draw_shape"), &mut self.lqf_dbg_draw_shape);
                    ui.checkbox(im_str!("lqf_dbg_draw_joint"), &mut self.lqf_dbg_draw_joint);
                    ui.checkbox(im_str!("lqf_dbg_draw_aabb"), &mut self.lqf_dbg_draw_aabb);
                    ui.checkbox(im_str!("lqf_dbg_draw_pair"), &mut self.lqf_dbg_draw_pair);
                    ui.checkbox(im_str!("lqf_dbg_draw_center_of_mass"), &mut self.lqf_dbg_draw_center_of_mass);
                    ui.checkbox(im_str!("lqf_dbg_draw_particle"), &mut self.lqf_dbg_draw_particle);
                    ui.unindent();
                // });
            });
            TreeNode::new(im_str!("display")).label(im_str!("display")).build(ui, || {
                ui.checkbox(im_str!("fullscreen"), &mut self.fullscreen);
                ui.set_next_item_width(110.0);
                ui.indent();
                ComboBox::new(im_str!("fullscreen_type")).build_simple_string(ui, &mut self.fullscreen_type, &[
                    im_str!("borderless"),
                    im_str!("fullscreen"),
                ]);
                ui.unindent();
                ui.checkbox(im_str!("vsync"), &mut self.vsync);
                ui.checkbox(im_str!("minimize_on_lost_focus"), &mut self.minimize_on_lost_focus);
            });
            TreeNode::new(im_str!("simulation")).label(im_str!("simulation")).build(ui, || {
                ui.checkbox(im_str!("tick"), &mut self.tick);
                ui.set_next_item_width(121.0);
                Slider::new(im_str!("tick_speed"))
                    .range(1..=120)
                    .flags(SliderFlags::ALWAYS_CLAMP)
                    .build(ui, &mut self.tick_speed);
                ui.same_line(0.0);
                if ui.small_button(im_str!("reset")) {
                    self.tick_speed = 30;
                }
                ui.checkbox(im_str!("load_chunks"), &mut self.load_chunks);
                ui.checkbox(im_str!("pause_on_lost_focus"), &mut self.pause_on_lost_focus);
            });
        });
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            draw_chunk_state_overlay: false,
            draw_chunk_state_overlay_alpha: 0.5,
            draw_chunk_dirty_rects: false,
            draw_chunk_grid: true,
            draw_origin: true,
            draw_load_zones: false,
            cull_chunks: true,
            lqf_dbg_draw: true,
            lqf_dbg_draw_shape: true,
            lqf_dbg_draw_joint: true,
            lqf_dbg_draw_aabb: true,
            lqf_dbg_draw_pair: true,
            lqf_dbg_draw_center_of_mass: true,
            lqf_dbg_draw_particle: true,

            fullscreen: false,
            fullscreen_type: 0,
            vsync: false,
            minimize_on_lost_focus: false,

            tick: true,
            tick_speed: 30,
            load_chunks: true,
            pause_on_lost_focus: false,
        }
    }
}