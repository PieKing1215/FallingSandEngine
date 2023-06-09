use std::{
    fs,
    path::{Path, PathBuf},
};

use asefile::AsepriteFile;
use itertools::Itertools;

use super::{
    asset_pack::AssetPack,
    modding::{ModRoot, ModRootError},
};

pub struct FileHelper {
    game_dir: PathBuf,
    asset_dir: PathBuf,
    asset_packs_root: PathBuf,
    asset_packs: Vec<AssetPack>,
}

impl FileHelper {
    pub fn new(game_dir: PathBuf, asset_dir: PathBuf, asset_packs_root: PathBuf) -> Self {
        Self {
            game_dir,
            asset_dir,
            asset_packs_root,
            asset_packs: vec![],
        }
    }

    pub fn load_asset_packs(&mut self) {
        let asset_pack_dirs = fs::read_dir(&self.asset_packs_root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                p.is_dir().then_some(p)
            })
            .filter(|p| {
                !p.ends_with(".disabled")
                    && p.extension().and_then(std::ffi::OsStr::to_str) != Some("disabled")
            })
            .collect::<Vec<_>>();

        let mut asset_packs = Vec::new();

        for dir in asset_pack_dirs {
            log::info!("Loading {dir:?}...");
            match AssetPack::load(&dir) {
                Ok(pack) => asset_packs.push(pack),
                Err(e) => log::error!("Error loading pack at {dir:?}: {e:?}"),
            }
        }

        self.asset_packs = asset_packs;
    }

    pub fn game_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.game_dir.join(path)
    }

    pub fn asset_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.asset_dir.join(path)
    }

    pub fn asset_packs_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.asset_packs_root.join(path)
    }

    pub fn read_asset<P: AsRef<Path>>(&self, path: P) -> Option<Vec<u8>> {
        if path.as_ref().to_str().unwrap() == "texture/material/smooth_stone_128x.png" {
            log::debug!(
                "here {} {}",
                path.as_ref().to_str().unwrap(),
                self.asset_packs.len()
            );
            for ap in &self.asset_packs {
                log::debug!("path {:?}", ap.asset_path(&path));
                log::debug!(
                    "res = {:?}",
                    fs::read(ap.asset_path(&path)).map(|v| v.len())
                );
            }
        }
        self.asset_packs
            .iter()
            .map(|ap| ap.asset_path(&path))
            .chain(std::iter::once(self.asset_path(&path)))
            .find_map(|p| fs::read(p).ok())
    }

    pub fn read_asset_to_string<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        self.asset_packs
            .iter()
            .map(|ap| ap.asset_path(&path))
            .chain(std::iter::once(self.asset_path(&path)))
            .find_map(|p| fs::read_to_string(p).ok())
    }

    pub fn read_asset_to_aseprite<P: AsRef<Path>>(&self, path: P) -> Option<AsepriteFile> {
        self.asset_packs
            .iter()
            .map(|ap| ap.asset_path(&path))
            .chain(std::iter::once(self.asset_path(&path)))
            .find_map(|p| AsepriteFile::read_file(p.as_path()).ok())
    }

    pub fn files_in_dir<P: AsRef<Path>>(&self, path: P) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        let path2 = path.as_ref().to_path_buf();
        let asset_packs = self
            .asset_packs
            .iter()
            .flat_map(move |ap| ap.files_in_dir(&path2));
        let assets = fs::read_dir(self.asset_path(&path))
            .into_iter()
            .flat_map(|dir| {
                dir.flatten().flat_map(|entry| {
                    let p = entry.path();
                    p.strip_prefix(&self.asset_dir)
                        .map(|rel| (p.clone(), rel.into()))
                })
            });
        let all_unique = asset_packs
            .chain(assets)
            // at this point, items are (PathBuf, PathBuf), where first is absolute and second is relative to pack roots
            // we deduplicate (keep first) by relative paths so that packs can override each other's files
            .dedup_by(|(_, a), (_, b)| a == b)
            .map(|(abs, _rel)| abs);

        Box::new(all_unique)
    }

    pub fn files_in_dir_with_ext<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        extension: &'a str,
    ) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        Box::new(self.files_in_dir(path).filter(move |p| {
            p.extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case(extension))
        }))
    }

    pub fn mod_files<'a>(&'a self) -> Box<dyn Iterator<Item = ModRoot> + '_> {
        Box::new(
            fs::read_dir(self.game_path("mods"))
                .into_iter()
                .flat_map(|dir| dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
                .filter_map(move |p| match ModRoot::try_from(p.clone()) {
                    Ok(mr) => Some(mr),
                    Err(ModRootError::NotDirOrZip(_)) => None,
                    Err(e) => {
                        log::error!("Error reading mod root @ {p:?}: {e:?}");
                        None
                    },
                }),
        )
    }
}
