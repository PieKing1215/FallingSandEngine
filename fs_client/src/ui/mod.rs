pub mod draw;
mod main_menu;

use fs_common::game::Registries;
pub use main_menu::*;

use self::draw::DrawUI;

pub struct DebugUIs {
    pub draw: DrawUI,
}

impl DebugUIs {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { draw: DrawUI::new() }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, registries: &Registries) {
        self.draw.render(egui_ctx, registries);
    }
}
