use std::path::PathBuf;

use imgui::WindowFlags;

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
}

impl MainMenu {
    fn draw_worlds(
        tree: &WorldTreeNode<PathBuf, (PathBuf, WorldMeta)>,
        ui: &imgui::Ui,
    ) -> Option<PathBuf> {
        match tree {
            WorldTreeNode::Folder(p, ch) => {
                // TODO: actually implement collapsing
                if !ui.button_with_size(
                    p.file_name()
                        .map_or_else(|| "..", |o| o.to_str().unwrap_or("!! NON UTF-8 !!")),
                    [200.0, 20.0],
                ) {
                    ui.indent();
                    for tr in ch {
                        let res = Self::draw_worlds(tr, ui);
                        if res.is_some() {
                            return res;
                        }
                    }
                    ui.unindent();
                }
            },
            WorldTreeNode::World((p, m)) => {
                if ui.button_with_size(
                    format!(
                        "{}\n{} - {}",
                        m.name,
                        p.parent()
                            .expect("World file missing parent folder ??")
                            .file_name()
                            .map_or_else(|| "..", |o| o.to_str().unwrap_or("!! NON UTF-8 !!")),
                        m.last_played_time
                    ),
                    [300.0, 40.0],
                ) {
                    return Some(p.clone());
                }
            },
        }
        None
    }

    pub fn render(&mut self, ui: &imgui::Ui, file_helper: &FileHelper) {
        ui.window("Main Menu")
            .size([300.0, 600.0], imgui::Condition::FirstUseEver)
            .flags(WindowFlags::ALWAYS_AUTO_RESIZE)
            .resizable(false)
            .position([400.0, 200.0], imgui::Condition::FirstUseEver)
            .build(|| {
                let mut new_state = None;
                match &self.state {
                    MainMenuState::Main => {
                        if ui.button_with_size("Singleplayer", [100.0, 50.0]) {
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
                        if ui.button_with_size("Quit", [100.0, 50.0]) {
                            self.action_queue.push(MainMenuAction::Quit);
                        }
                    },
                    MainMenuState::WorldSelect { context } => {
                        if ui.button_with_size("Back", [50.0, 20.0]) {
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
