use egui::TextureOptions;
use fs_common::game::common::world::material::buf::MaterialBuf;

use super::DebugUIsContext;

pub struct ClipboardUI {
    texture: Option<(MaterialBuf, egui::TextureHandle)>,
}

impl ClipboardUI {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { texture: None }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, ctx: &mut DebugUIsContext) {
        if let Some(c) = &ctx.local_player.clipboard.clipboard {
            if self.texture.as_ref().map_or(true, |(prev, _)| prev != c) {
                self.texture = Some((
                    c.clone(),
                    egui_ctx.load_texture(
                        "clipboard preview",
                        gen_preview(c),
                        TextureOptions::LINEAR,
                    ),
                ));
            }
        } else {
            self.texture = None;
        }

        egui::Window::new("Clipboard")
            .resizable(false)
            .show(egui_ctx, |ui| {
                if let Some((_, tex)) = &self.texture {
                    egui::ScrollArea::both()
                        .max_width(400.0)
                        .max_height(400.0)
                        .show(ui, |ui| {
                            ui.image(tex, tex.size_vec2());
                        });

                    ui.label(format!("state: {:?}", ctx.local_player.clipboard.state));

                    if ui.button("Clear").clicked() {
                        ctx.local_player.clipboard.clear();
                    }
                } else {
                    ui.label("Nothing here...");
                }
            });
    }
}

fn gen_preview(buf: &MaterialBuf) -> egui::ColorImage {
    let width = buf.width as usize;
    let height = buf.height as usize;
    let fake_nearest_neighbor_scale = if width < 100 && height < 100 {
        3
    } else if width < 200 && height < 200 {
        2
    } else {
        1
    };
    let mut img = egui::ColorImage::new(
        [
            width * fake_nearest_neighbor_scale,
            height * fake_nearest_neighbor_scale,
        ],
        egui::Color32::TRANSPARENT,
    );
    for y in 0..height {
        for x in 0..width {
            let mat = buf.materials[x + y * width].clone();
            let col = egui::Rgba::from_srgba_unmultiplied(
                mat.color.r,
                mat.color.g,
                mat.color.b,
                mat.color.a,
            )
            .into();

            for xx in 0..fake_nearest_neighbor_scale {
                for yy in 0..fake_nearest_neighbor_scale {
                    img[(
                        x * fake_nearest_neighbor_scale + xx,
                        y * fake_nearest_neighbor_scale + yy,
                    )] = col;
                }
            }
        }
    }
    img
}
