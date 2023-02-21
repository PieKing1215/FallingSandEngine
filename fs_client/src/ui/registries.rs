use egui::{
    epaint::ahash::HashMap,
    plot::{Arrows, Plot, PlotImage, PlotPoint, PlotPoints, Points, Text},
    Color32, RichText, ScrollArea, TextureOptions, Vec2,
};
use fs_common::game::common::{
    registry::RegistryID,
    world::{copy_paste::MaterialBuf, gen::structure::piece::StructurePiece},
};

use super::DebugUIsContext;

pub struct RegistriesUI {
    cur_tab: Tab,
    structure_piece_images: HashMap<RegistryID<StructurePiece>, egui::TextureHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Material,
    MaterialPlacer,
    StructurePiece,
    StructurePool,
    ConfiguredStructure,
    StructureSet,
}

impl RegistriesUI {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            cur_tab: Tab::Material,
            structure_piece_images: HashMap::default(),
        }
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, ctx: &mut DebugUIsContext) {
        egui::Window::new("Registries")
            .resizable(false)
            .show(egui_ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.cur_tab, Tab::Material, "Material");
                    ui.selectable_value(&mut self.cur_tab, Tab::MaterialPlacer, "MaterialPlacer");
                    ui.selectable_value(&mut self.cur_tab, Tab::StructurePiece, "StructurePiece");
                    ui.selectable_value(&mut self.cur_tab, Tab::StructurePool, "StructurePool");
                    ui.selectable_value(
                        &mut self.cur_tab,
                        Tab::ConfiguredStructure,
                        "ConfiguredStructure",
                    );
                    ui.selectable_value(&mut self.cur_tab, Tab::StructureSet, "StructureSet");
                });

                match self.cur_tab {
                    Tab::Material => {
                        for (id, mat) in &ctx.registries.materials {
                            ui.collapsing(format!("{id}"), |ui| {
                                ui.label(format!("display_name = {}", mat.display_name));
                            });
                        }
                    },
                    Tab::MaterialPlacer => {
                        for (id, placer) in &ctx.registries.material_placers {
                            ui.collapsing(format!("{id}"), |ui| {
                                ui.label(format!("display_name = {}", placer.meta.display_name));
                            });
                        }
                    },
                    Tab::StructurePiece => {
                        let mut entries: Vec<_> =
                            (&ctx.registries.structure_pieces).into_iter().collect();
                        entries.sort_by(|(k_a, _), (k_b, _)| k_a.cmp(k_b));
                        for (id, piece) in entries {
                            ui.collapsing(format!("{id}"), |ui| {
                                let tex = self
                                    .structure_piece_images
                                    .entry(id.clone())
                                    .or_insert_with(|| {
                                        egui_ctx.load_texture(
                                            "structure piece preview",
                                            gen_preview(&piece.buf),
                                            TextureOptions::LINEAR,
                                        )
                                    });
                                let size = tex.size_vec2();
                                let margin = 24.0;
                                Plot::new("plot")
                                    .width(size.x + margin)
                                    .height(size.y + margin)
                                    .allow_drag(false)
                                    .allow_boxed_zoom(false)
                                    .allow_scroll(false)
                                    .allow_zoom(false)
                                    // .show_background(false)
                                    .set_margin_fraction(Vec2::new(
                                        margin / (size.x + margin),
                                        margin / (size.y + margin),
                                    ))
                                    .show(ui, |ui| {
                                        ui.image(PlotImage::new(
                                            tex,
                                            PlotPoint::new(size.x / 2.0, -size.y / 2.0),
                                            size,
                                        ));
                                        let points_config: Vec<_> = piece
                                            .child_nodes
                                            .iter()
                                            .map(|(p, c)| (PlotPoint::new(p.x, -(p.y as f32)), c))
                                            .collect();
                                        let points: Vec<_> =
                                            points_config.iter().map(|(p, _)| *p).collect();
                                        let tips: Vec<_> = piece
                                            .child_nodes
                                            .iter()
                                            .map(|(p, _)| {
                                                PlotPoint::new(
                                                    p.x as f32
                                                        + p.direction_out.vec().0 as f32 * 10.0,
                                                    -(p.y as f32)
                                                        + -p.direction_out.vec().1 as f32 * 10.0,
                                                )
                                            })
                                            .collect();
                                        ui.points(
                                            Points::new(PlotPoints::Owned(points.clone()))
                                                .radius(1.5),
                                        );
                                        ui.arrows(
                                            Arrows::new(
                                                PlotPoints::Owned(points),
                                                PlotPoints::Owned(tips),
                                            )
                                            .highlight(true),
                                        );
                                        for (i, (p, _)) in points_config.iter().enumerate() {
                                            ui.text(Text::new(
                                                *p,
                                                RichText::new(format!("{i}"))
                                                    .size(14.0)
                                                    .color(Color32::WHITE),
                                            ))
                                        }
                                    });

                                for (i, (p, c)) in piece.child_nodes.iter().enumerate() {
                                    ui.collapsing(format!("connection #{i}"), |ui| {
                                        ui.label(format!("pos = ({}, {})", p.x, p.y));
                                        ui.label(format!("pool = {:?}", c.pool));
                                        ui.label(format!(
                                            "depth_override = {:?}",
                                            c.depth_override
                                        ));
                                        ui.label(format!("block_in_dirs = {:?}", c.block_in_dirs));
                                    });
                                }
                            });
                        }
                    },
                    Tab::StructurePool => {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            for (id, pool) in &ctx.registries.structure_pools {
                                ui.collapsing(format!("{id}"), |ui| {
                                    ui.label(format!("pool = {:#?}", pool.pool));
                                });
                            }
                        });
                    },
                    Tab::ConfiguredStructure => {
                        for (id, configured) in &ctx.registries.configured_structures {
                            ui.collapsing(format!("{id}"), |ui| {
                                ui.label(format!("placer = {:#?}", configured.placer));
                            });
                        }
                    },
                    Tab::StructureSet => {
                        for (id, set) in &ctx.registries.structure_sets {
                            ui.collapsing(format!("{id}"), |ui| {
                                ui.label(format!("structures = {:?}", set.structures));
                                ui.label(format!("frequency = {}", set.frequency));
                                ui.label(format!("exclusion = {:?}", set.exclusion));
                                ui.label(format!("spacing = {}", set.spacing));
                                ui.label(format!("separation = {}", set.separation));
                                ui.label(format!("salt = {}", set.salt));
                            });
                        }
                    },
                }
            });
    }
}

pub fn gen_preview(buf: &MaterialBuf) -> egui::ColorImage {
    let width = buf.width as usize;
    let height = buf.height as usize;
    let fake_nearest_neighbor_scale = 1;
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
