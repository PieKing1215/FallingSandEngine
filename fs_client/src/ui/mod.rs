mod main_menu;
pub mod draw;

pub use main_menu::*;

use self::draw::DrawUI;

pub struct DebugUIs {
    draw: DrawUI,
}

impl DebugUIs {
    pub fn new() -> Self {
        Self {
            draw: DrawUI::new(),
        }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context) {
        self.draw.render(egui_ctx);
    }
}