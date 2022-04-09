use std::collections::BTreeMap;

use fs_common::game::common::world::material::{self, Material};

pub struct DrawUI {
    textures: BTreeMap<u16, egui::TextureHandle>,
    selected: u16,
}

impl DrawUI {
    pub fn new() -> Self {
        Self {
            textures: BTreeMap::new(),
            selected: material::AIR.id,
        }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context) {
        self.textures.entry(material::AIR.id).or_insert_with(|| egui_ctx.load_texture("my-image", gen_material_preview(&material::AIR)));
        self.textures.entry(material::TEST_MATERIAL.id).or_insert_with(|| egui_ctx.load_texture("my-image", gen_material_preview(&material::TEST_MATERIAL)));

        egui::Window::new("Draw")
            .resizable(false)
            .show(egui_ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Min).with_main_wrap(true), |ui| {
                    for (id, tex) in &self.textures {
                        if ui.add(egui::ImageButton::new(tex, (32.0, 32.0)).selected(*id == self.selected))
                            .on_hover_text(format!("{id}"))
                            .clicked() {
                            self.selected = *id;
                        };
                    }
                });
            });
    }
}

fn gen_material_preview(mat: &Material) -> egui::ColorImage {
    let width = 8;
    let height = 8;
    let fake_nearest_neighbor_scale = 4;
    let mut img = egui::ColorImage::new([width * fake_nearest_neighbor_scale, height * fake_nearest_neighbor_scale], egui::Color32::TRANSPARENT);
    for y in 0..height {
        for x in 0..width {
            let h = x as f32 / width as f32;
            let s = 1.0;
            let v = 1.0;
            let a = y as f32 / height as f32;
            for xx in 0..fake_nearest_neighbor_scale {
                for yy in 0..fake_nearest_neighbor_scale {
                    img[(x * fake_nearest_neighbor_scale + xx, y * fake_nearest_neighbor_scale + yy)] = egui::color::Hsva { h, s, v, a }.into();
                }
            }
        }
    }
    img
}
