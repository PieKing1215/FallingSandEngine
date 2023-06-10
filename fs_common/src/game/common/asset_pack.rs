use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use super::dir_or_zip::{DirOrZip, ReadFile, RelativePathBuf};

pub struct AssetPack {
    root: DirOrZip,
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
    #[error("missing pack.ron")]
    MissingPackMeta,
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("meta deserialize error")]
    MetaError(#[from] ron::de::SpannedError),
}

impl AssetPack {
    pub fn load(root: DirOrZip) -> Result<Self, AssetPackLoadError> {
        let meta = root
            .read_file("pack.ron")
            .ok_or(AssetPackLoadError::MissingPackMeta)?;
        Ok(AssetPack { meta: ron::de::from_bytes(&meta)?, root })
    }

    pub fn meta(&self) -> &AssetPackMeta {
        &self.meta
    }

    pub fn file<P: AsRef<Path>>(&self, path: P) -> Option<Box<dyn ReadFile + '_>> {
        self.root.file(path)
    }

    pub fn iter_dir<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
    ) -> Box<dyn Iterator<Item = (Box<dyn ReadFile + '_>, RelativePathBuf)> + '_> {
        self.root.iter_dir(path)
    }
}
