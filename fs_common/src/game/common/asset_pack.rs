use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

pub struct AssetPack {
    dir: PathBuf,
    meta: AssetPackMeta,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "AssetPack")]
pub struct AssetPackMeta {
    pub display_name: String,
    pub description: String,
    #[allow(dead_code)]
    format_version: u16,
}

#[derive(Error, Debug)]
pub enum AssetPackLoadError {
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("meta deserialize error")]
    MetaError(#[from] ron::de::SpannedError),
}

impl AssetPack {
    pub fn load(dir: &Path) -> Result<Self, AssetPackLoadError> {
        let meta = fs::read(dir.join("pack.ron"))?;
        Ok(AssetPack {
            meta: ron::de::from_bytes(&meta)?,
            dir: dir.to_path_buf(),
        })
    }

    pub fn meta(&self) -> &AssetPackMeta {
        &self.meta
    }

    pub fn asset_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.dir.join(path)
    }

    /// First `PathBuf` is absolute, second is relative to pack root
    pub fn files_in_dir<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Box<dyn Iterator<Item = (PathBuf, PathBuf)> + '_> {
        Box::new(
            fs::read_dir(self.asset_path(path))
                .into_iter()
                .flat_map(|dir| {
                    dir.flatten()
                        .flat_map(|entry| {
                            let p = entry.path();
                            p.strip_prefix(&self.dir).map(|rel| (p.clone(), rel.into()))
                        })
                        .collect::<Vec<_>>()
                }),
        )
    }
}
