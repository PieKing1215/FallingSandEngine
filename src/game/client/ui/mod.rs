
mod main_menu;

pub use main_menu::*;

pub trait UI {
    fn render(&mut self, ui: &imgui::Ui);
}