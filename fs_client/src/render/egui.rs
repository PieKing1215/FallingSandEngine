use std::sync::Arc;

use fs_common::game::{common::Settings, Registries};

pub trait DebugUI {
    fn debug_ui(&mut self, ui: &mut egui::Ui, registries: Arc<Registries>);
}

impl DebugUI for Settings {
    #[profiling::function]
    fn debug_ui(&mut self, ui: &mut egui::Ui, registries: Arc<Registries>) {
        ui.collapsing("rendering", |ui| {
            ui.checkbox(
                &mut self.draw_chunk_state_overlay,
                "draw_chunk_state_overlay",
            );
            if self.draw_chunk_state_overlay {
                ui.indent("draw_chunk_state_overlay#indent", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.draw_chunk_state_overlay_alpha, 0.1..=1.0)
                            .text("alpha")
                            .clamp_to_range(true),
                    );
                });
            }

            ui.checkbox(&mut self.draw_chunk_dirty_rects, "draw_chunk_dirty_rects");
            ui.checkbox(&mut self.draw_chunk_grid, "draw_chunk_grid");
            ui.checkbox(&mut self.draw_origin, "draw_origin");
            ui.checkbox(&mut self.draw_load_zones, "draw_load_zones");
            ui.checkbox(&mut self.draw_structure_bounds, "draw_structure_bounds");

            let mut opt = vec![("none", None)];
            for (k, v) in &registries.structure_sets {
                opt.push((k, Some(v)));
            }
            egui::ComboBox::from_label("draw_structure_set")
                .selected_text(self.draw_structure_set.unwrap_or("none"))
                .show_ui(ui, |ui| {
                    for (k, v) in opt {
                        ui.selectable_value(&mut self.draw_structure_set, v.map(|_| k), k);
                    }
                });

            ui.checkbox(&mut self.cull_chunks, "cull_chunks");

            let opt = [
                "none",
                "marching squares",
                "ramer douglas peucker",
                "earcutr",
            ];
            egui::ComboBox::from_label("draw_chunk_collision")
                .selected_text(opt[self.draw_chunk_collision])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draw_chunk_collision, 0, opt[0]);
                    ui.selectable_value(&mut self.draw_chunk_collision, 1, opt[1]);
                    ui.selectable_value(&mut self.draw_chunk_collision, 2, opt[2]);
                    ui.selectable_value(&mut self.draw_chunk_collision, 3, opt[3]);
                });

            ui.checkbox(&mut self.physics_dbg_draw, "physics_dbg_draw");

            ui.indent("physics_dbg_draw#indent", |ui| {
                ui.checkbox(&mut self.physics_dbg_draw_shape, "shape");
                ui.checkbox(&mut self.physics_dbg_draw_joint, "joint");
                ui.checkbox(&mut self.physics_dbg_draw_aabb, "aabb");
                ui.checkbox(&mut self.physics_dbg_draw_pair, "pair");
                ui.checkbox(&mut self.physics_dbg_draw_center_of_mass, "center_of_mass");
                ui.checkbox(&mut self.physics_dbg_draw_particle, "particle");
            });
        });

        ui.collapsing("display", |ui| {
            ui.checkbox(&mut self.fullscreen, "fullscreen");

            ui.indent("fullscreen#indent", |ui| {
                let opt = ["borderless", "fullscreen"];
                egui::ComboBox::from_label("fullscreen_type")
                    .selected_text(opt[self.fullscreen_type])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fullscreen_type, 0, opt[0]);
                        ui.selectable_value(&mut self.fullscreen_type, 1, opt[1]);
                    });
            });

            ui.checkbox(&mut self.vsync, "vsync");
            ui.checkbox(&mut self.minimize_on_lost_focus, "minimize_on_lost_focus");
        });

        ui.collapsing("simulation", |ui| {
            ui.checkbox(&mut self.tick, "tick");
            ui.indent("tick#indent", |ui| {
                ui.add(
                    egui::Slider::new(&mut self.tick_speed, 1..=120)
                        .text("tick_speed")
                        .clamp_to_range(true),
                );

                if ui.button("reset##tick_speed").clicked() {
                    self.tick_speed = 30;
                }
            });

            ui.checkbox(&mut self.tick_physics, "tick_physics");
            ui.indent("tick_physics#indent", |ui| {
                ui.add(
                    egui::Slider::new(&mut self.tick_physics_speed, 1..=120)
                        .text("tick_physics_speed")
                        .clamp_to_range(true),
                );

                if ui.button("reset##tick_physics_speed").clicked() {
                    self.tick_physics_speed = 60;
                }

                ui.add(
                    egui::Slider::new(&mut self.tick_physics_timestep, 0.01..=1.0)
                        .text("tick_physics_timestep")
                        .clamp_to_range(true),
                );
                if ui.button("reset##tick_physics_timestep").clicked() {
                    self.tick_physics_timestep = 1.0 / 45.0;
                }
            });

            ui.checkbox(&mut self.load_chunks, "load_chunks");
            ui.checkbox(&mut self.simulate_chunks, "simulate_chunks");
            ui.checkbox(&mut self.simulate_particles, "simulate_particles");
            ui.checkbox(&mut self.pause_on_lost_focus, "pause_on_lost_focus");
        });
    }
}
