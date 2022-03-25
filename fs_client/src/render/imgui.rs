use fs_common::game::common::Settings;
use imgui::{SliderFlags, WindowFlags};

pub trait DebugUI {
    fn debug_ui(&mut self, ui: &imgui::Ui);
}

impl DebugUI for Settings {
    #[profiling::function]
    fn debug_ui(&mut self, ui: &imgui::Ui) {
        ui.window("Debug Menu")
            .size([300.0, 600.0], imgui::Condition::FirstUseEver)
            .flags(WindowFlags::ALWAYS_AUTO_RESIZE)
            .resizable(false)
            .build(|| {
                ui.tree_node_config("rendering").build(|| {
                    // TreeNode::new("chunk_overlay").label("chunk overlay").build(ui, || {
                    ui.checkbox(
                        "draw_chunk_state_overlay",
                        &mut self.draw_chunk_state_overlay,
                    );
                    if self.draw_chunk_state_overlay {
                        ui.indent();
                        ui.set_next_item_width(80.0);
                        ui.slider_config("alpha", 0.1, 1.0)
                            .display_format("%.1f")
                            .flags(SliderFlags::ALWAYS_CLAMP)
                            .build(&mut self.draw_chunk_state_overlay_alpha);
                        ui.unindent();
                    }
                    ui.checkbox("draw_chunk_dirty_rects", &mut self.draw_chunk_dirty_rects);
                    ui.checkbox("draw_chunk_grid", &mut self.draw_chunk_grid);
                    ui.checkbox("draw_origin", &mut self.draw_origin);
                    ui.checkbox("draw_load_zones", &mut self.draw_load_zones);
                    ui.checkbox("cull_chunks", &mut self.cull_chunks);

                    ui.combo_simple_string(
                        "draw_chunk_collision",
                        &mut self.draw_chunk_collision,
                        &[
                            "none",
                            "marching squares",
                            "ramer douglas peucker",
                            "earcutr",
                        ],
                    );

                    ui.checkbox("physics_dbg_draw", &mut self.physics_dbg_draw);
                    ui.indent();
                    ui.checkbox("shape", &mut self.physics_dbg_draw_shape);
                    ui.checkbox("joint", &mut self.physics_dbg_draw_joint);
                    ui.checkbox("aabb", &mut self.physics_dbg_draw_aabb);
                    ui.checkbox("pair", &mut self.physics_dbg_draw_pair);
                    ui.checkbox("center_of_mass", &mut self.physics_dbg_draw_center_of_mass);
                    ui.checkbox("particle", &mut self.physics_dbg_draw_particle);
                    ui.unindent();
                    // });
                });
                ui.tree_node_config("display").build(|| {
                    ui.checkbox("fullscreen", &mut self.fullscreen);
                    ui.set_next_item_width(110.0);
                    ui.indent();
                    ui.combo_simple_string(
                        "fullscreen_type",
                        &mut self.fullscreen_type,
                        &["borderless", "fullscreen"],
                    );
                    ui.unindent();
                    ui.checkbox("vsync", &mut self.vsync);
                    ui.checkbox("minimize_on_lost_focus", &mut self.minimize_on_lost_focus);
                });
                ui.tree_node_config("simulation").build(|| {
                    ui.checkbox("tick", &mut self.tick);

                    ui.indent();
                    ui.set_next_item_width(121.0);
                    ui.slider_config("tick_speed", 1, 120)
                        .flags(SliderFlags::ALWAYS_CLAMP)
                        .build(&mut self.tick_speed);
                    ui.same_line();
                    if ui.small_button("reset##tick_speed") {
                        self.tick_speed = 30;
                    }
                    ui.unindent();

                    ui.checkbox("tick_physics", &mut self.tick_physics);

                    ui.indent();
                    ui.set_next_item_width(121.0);
                    ui.slider_config("tick_physics_speed", 1, 120)
                        .flags(SliderFlags::ALWAYS_CLAMP)
                        .build(&mut self.tick_physics_speed);
                    ui.same_line();
                    if ui.small_button("reset##tick_physics_speed") {
                        self.tick_physics_speed = 60;
                    }

                    ui.set_next_item_width(121.0);
                    ui.slider_config("tick_physics_timestep", 0.01, 1.0)
                        .flags(SliderFlags::ALWAYS_CLAMP)
                        .build(&mut self.tick_physics_timestep);
                    ui.same_line();
                    if ui.small_button("reset##tick_physics_timestep") {
                        self.tick_physics_timestep = 1.0 / 45.0;
                    }
                    ui.unindent();

                    ui.checkbox("load_chunks", &mut self.load_chunks);
                    ui.checkbox("simulate_chunks", &mut self.simulate_chunks);
                    ui.checkbox("simulate_particles", &mut self.simulate_particles);
                    ui.checkbox("pause_on_lost_focus", &mut self.pause_on_lost_focus);
                });
            });
    }
}
