use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use asefile::AsepriteFile;
use itertools::Itertools;

use super::{
    asset_pack::AssetPack,
    dir_or_zip::{DirOrZip, DirOrZipError, PathBufExt, ReadEntry},
    modding::ModManager,
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
                (std::path::Path::is_dir(&p)
                    || std::path::Path::extension(&p).and_then(OsStr::to_str) == Some("zip"))
                .then_some(p)
            })
            .filter(|p| {
                !p.ends_with(".disabled")
                    && std::path::Path::extension(p).and_then(OsStr::to_str) != Some("disabled")
            })
            .collect::<Vec<_>>();

        let mut asset_packs = Vec::new();

        for dir in asset_pack_dirs {
            log::info!("Loading {dir:?}...");
            let root = DirOrZip::try_from(dir.clone()).expect("not dir or zip");
            match AssetPack::load(root) {
                Ok(pack) => asset_packs.push(pack),
                Err(e) => log::error!("Error loading pack at {dir:?}: {e:?}"),
            }
        }

        self.asset_packs = asset_packs;
    }

    pub fn load_mod_asset_packs(&mut self, mod_manager: &ModManager) {
        for m in mod_manager.mods() {
            self.asset_packs.extend(m.load_asset_packs());
        }
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
        self.asset_packs
            .iter()
            .filter_map(|ap| ap.file(&path))
            .chain(std::iter::once(Box::new(self.asset_path(&path)) as _))
            .find_map(|mut f| f.read().ok())
    }

    pub fn read_asset_to_string<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        self.asset_packs
            .iter()
            .filter_map(|ap| ap.file(&path))
            .chain(std::iter::once(Box::new(self.asset_path(&path)) as _))
            .find_map(|mut f| f.read_to_string().ok())
    }

    pub fn read_asset_to_aseprite<P: AsRef<Path>>(&self, path: P) -> Option<AsepriteFile> {
        self.asset_packs
            .iter()
            .filter_map(|ap| ap.file(&path))
            .chain(std::iter::once(Box::new(self.asset_path(&path)) as _))
            .find_map(|mut f| {
                f.read()
                    .ok()
                    .and_then(|b| AsepriteFile::read(b.as_slice()).ok())
            })
    }

    pub fn files_in_dir<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
    ) -> Box<dyn Iterator<Item = Box<dyn ReadEntry + '_>> + '_> {
        let path2 = path.as_ref().to_path_buf();
        let asset_packs = self
            .asset_packs
            .iter()
            .flat_map(move |ap| ap.iter_dir(path2.clone()));

        let assets = self.asset_dir.iter_dir(path);
        let all_unique = asset_packs
            .chain(assets)
            // at this point, items are `(Box<dyn ReadFile>, RelativePathBuf)`
            // we deduplicate (keep first) by relative path so that packs can override each other's files
            .dedup_by(|(_, a), (_, b)| a == b)
            .map(|(file, _rel)| file);

        Box::new(all_unique)
    }

    pub fn files_in_dir_with_ext<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
        extension: &'a str,
    ) -> Box<dyn Iterator<Item = Box<dyn ReadEntry + '_>> + '_> {
        Box::new(self.files_in_dir(path).filter_map(move |mut p| {
            p.extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case(extension))
                .then_some(p)
        }))
    }

    pub fn mod_files<'a>(&'a self) -> Box<dyn Iterator<Item = DirOrZip> + '_> {
        Box::new(
            fs::read_dir(self.game_path("mods"))
                .into_iter()
                .flat_map(|dir| dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
                .filter(|p| {
                    !p.ends_with(".disabled")
                        && std::path::Path::extension(p).and_then(OsStr::to_str) != Some("disabled")
                })
                .filter_map(move |p| match DirOrZip::try_from(p) {
                    Ok(mr) => Some(mr),
                    Err(DirOrZipError::NotDirOrZip(_)) => None,
                }),
        )
    }
}
