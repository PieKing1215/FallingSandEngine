use std::path::{Path, PathBuf};

pub struct FileHelper {
    game_dir: PathBuf,
    asset_dir: PathBuf,
}

impl FileHelper {
    pub fn new(game_dir: PathBuf, asset_dir: PathBuf) -> Self {
        Self {
            game_dir,
            asset_dir,
        }
    }

    pub fn game_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.game_dir.join(path)
    }

    pub fn asset_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.asset_dir.join(path)
    }
}