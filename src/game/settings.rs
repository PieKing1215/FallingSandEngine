use imgui::{Slider, TreeNode, WindowFlags, im_str};


pub struct Settings {
    pub draw_chunk_state_overlay: bool,
    pub draw_chunk_state_overlay_alpha: f32,
    pub draw_chunk_grid: bool,
    pub draw_origin: bool,
    pub draw_load_zones: bool,
    pub cull_chunks: bool,
}

impl Settings {
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
                            .build(ui, &mut self.draw_chunk_state_overlay_alpha);
                        ui.unindent();
                    }
                    ui.checkbox(im_str!("draw_chunk_grid"), &mut self.draw_chunk_grid);
                    ui.checkbox(im_str!("draw_origin"), &mut self.draw_origin);
                    ui.checkbox(im_str!("draw_load_zones"), &mut self.draw_load_zones);
                    ui.checkbox(im_str!("cull_chunks"), &mut self.cull_chunks);
                // });
            });
            
        });
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            draw_chunk_state_overlay: false,
            draw_chunk_state_overlay_alpha: 0.5,
            draw_chunk_grid: true,
            draw_origin: true,
            draw_load_zones: false,
            cull_chunks: true,
        }
    }
}