use std::path::PathBuf;

use fs_common::game::{
    self,
    common::{
        world::{WorldMeta, WorldTreeNode},
        FileHelper,
    },
};

use crate::world::ClientChunk;

pub struct MainMenu {
    pub state: MainMenuState,
    pub action_queue: Vec<MainMenuAction>,
}

pub enum MainMenuState {
    Main,
    WorldSelect {
        context: WorldTreeNode<PathBuf, (PathBuf, WorldMeta)>,
    },
}

pub enum MainMenuAction {
    Quit,
    LoadWorld(PathBuf),
    LoadRandomSeed,
}

impl MainMenu {
    fn draw_worlds(
        tree: &WorldTreeNode<PathBuf, (PathBuf, WorldMeta)>,
        ui: &mut egui::Ui,
    ) -> Option<PathBuf> {
        match tree {
            WorldTreeNode::Folder(p, ch) => {
                // TODO: actually implement collapsing
                if !ui
                    .button(
                        p.file_name()
                            .map_or_else(|| "..", |o| o.to_str().unwrap_or("!! NON UTF-8 !!")),
                    )
                    .clicked()
                {
                    let res = ui
                        .indent("a", |ui| {
                            for tr in ch {
                                let res = Self::draw_worlds(tr, ui);
                                if res.is_some() {
                                    return res;
                                }
                            }
                            None
                        })
                        .inner;

                    if res.is_some() {
                        return res;
                    }
                }
            },
            WorldTreeNode::World((p, m)) => {
                if ui
                    .button(format!(
                        "{}\n{} - {}",
                        m.name,
                        p.parent()
                            .expect("World file missing parent folder ??")
                            .file_name()
                            .map_or_else(|| "..", |o| o.to_str().unwrap_or("!! NON UTF-8 !!")),
                        m.last_played_time
                    ))
                    .clicked()
                {
                    return Some(p.clone());
                }
            },
        }
        None
    }

    pub fn render(&mut self, egui_ctx: &egui::Context, file_helper: &FileHelper) {
        egui::Window::new("Main Menu")
            .resizable(false)
            .show(egui_ctx, |ui| {
                let mut new_state = None;
                match &self.state {
                    MainMenuState::Main => {
                        if ui.button("Singleplayer").clicked() {
                            let worlds = game::common::world::World::<ClientChunk>::find_files(
                                file_helper.game_path("saves/"),
                            )
                            .expect("Failed to load worlds list");
                            log::debug!("{:?}", worlds);
                            let metas =
                                game::common::world::World::<ClientChunk>::parse_file_tree_metas(
                                    worlds,
                                )
                                .expect("World meta parse failed");
                            log::debug!("{:?}", metas);

                            new_state = Some(MainMenuState::WorldSelect { context: metas });
                        }
                        if ui.button("Random Seed").clicked() {
                            self.action_queue.push(MainMenuAction::LoadRandomSeed);
                        }
                        if ui.button("Quit").clicked() {
                            self.action_queue.push(MainMenuAction::Quit);
                        }
                    },
                    MainMenuState::WorldSelect { context } => {
                        if ui.button("Back").clicked() {
                            new_state = Some(MainMenuState::Main);
                        }
                        if let Some(choice) = Self::draw_worlds(context, ui) {
                            log::debug!("Chose world: {:?}", choice);
                            self.action_queue.push(MainMenuAction::LoadWorld(choice));
                        }
                    },
                }

                if let Some(new_state) = new_state {
                    self.state = new_state;
                }
            });
    }
}
