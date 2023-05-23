use egui::{collapsing_header::CollapsingState, Id};

use super::DebugUIsContext;

pub struct ModsUI {}

impl ModsUI {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, ctx: &mut DebugUIsContext) {
        // hack to default the window to collapsed
        // a fn for this was added in egui 0.21 but we need to use 0.20 for egui_glium
        // (could use the github for egui_glium but it uses winit 0.28 while glutin 0.29 uses wini 0.27 (glutin can bumped but might be a bit of work))
        let id = Id::new("Mods");
        CollapsingState::load_with_default_open(egui_ctx, id.with("collapsing"), false)
            .store(egui_ctx);

        egui::Window::new("Mods")
            .id(id)
            .resizable(false)
            .show(egui_ctx, |ui| {
                for m in ctx.mod_manager.mods_mut() {
                    let v = m.test_fn();
                    ui.label(v);
                }
            });
    }
}
