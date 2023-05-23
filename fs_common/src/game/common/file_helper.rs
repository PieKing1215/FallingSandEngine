use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct FileHelper {
    game_dir: PathBuf,
    asset_dir: PathBuf,
}

impl FileHelper {
    pub fn new(game_dir: PathBuf, asset_dir: PathBuf) -> Self {
        Self { game_dir, asset_dir }
    }

    pub fn game_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.game_dir.join(path)
    }

    pub fn asset_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.asset_dir.join(path)
    }

    pub fn files_in_dir<P: AsRef<Path>>(&self, path: P) -> Box<dyn Iterator<Item = PathBuf>> {
        Box::new(
            fs::read_dir(self.asset_path(path))
                .into_iter()
                .flat_map(|dir| dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>()),
        )
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

    pub fn mod_files<'a>(&'a self) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        Box::new(
            fs::read_dir(self.game_path("mods"))
                .into_iter()
                .flat_map(|dir| dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
                .filter(move |p| {
                    p.extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("wasm"))
                }),
        )
    }
}
