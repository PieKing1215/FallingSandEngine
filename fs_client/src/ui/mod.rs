pub mod clipboard;
pub mod draw;
mod main_menu;

use fs_common::game::{common::world::entity::Player, Registries};
pub use main_menu::*;

use self::{clipboard::ClipboardUI, draw::DrawUI};

pub struct DebugUIs {
    pub draw: DrawUI,
    pub clipboard: ClipboardUI,
}

pub struct DebugUIsContext<'a> {
    pub registries: &'a Registries,
    pub local_player: &'a mut Player,
}

impl DebugUIs {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { draw: DrawUI::new(), clipboard: ClipboardUI::new() }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, mut ctx: DebugUIsContext) {
        self.draw.render(egui_ctx, &ctx);
        self.clipboard.render(egui_ctx, &mut ctx);
    }
}
