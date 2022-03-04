use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{Chunk, World};

#[derive(Debug)]
pub enum WorldTreeNode<F, T> {
    Folder(F, Vec<WorldTreeNode<F, T>>),
    World(T),
}

#[derive(Debug, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub last_played_version: String,
    pub save_format: String,
    pub last_played_time: toml::value::Datetime,
}

impl<'w, C: Chunk> World<C> {
    pub fn find_files(root: PathBuf) -> Result<WorldTreeNode<PathBuf, PathBuf>, std::io::Error> {
        let mut res = Vec::new();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;

            if entry.path().is_dir() {
                res.push(Self::find_files(entry.path())?);
            } else if entry.file_name() == "world_info.toml" {
                return Ok(WorldTreeNode::World(entry.path()));
            }
        }

        Ok(WorldTreeNode::Folder(root, res))
    }

    pub fn parse_file_tree_metas(
        tree: WorldTreeNode<PathBuf, PathBuf>,
    ) -> Result<WorldTreeNode<PathBuf, (PathBuf, WorldMeta)>, Box<dyn std::error::Error>> {
        Ok(match tree {
            WorldTreeNode::Folder(p, children) => {
                let r = children
                    .into_iter()
                    .map(Self::parse_file_tree_metas)
                    .collect::<Result<_, _>>();
                WorldTreeNode::Folder(p, r?)
            },
            WorldTreeNode::World(p) => {
                let m = Self::parse_file_meta(&p)?;
                WorldTreeNode::World((p, m))
            },
        })
    }

    pub fn parse_file_meta<P: AsRef<Path>>(
        path: P,
    ) -> Result<WorldMeta, Box<dyn std::error::Error>> {
        Ok(toml::from_str::<WorldMeta>(&fs::read_to_string(path)?)?)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::create(Some(path.as_ref().to_path_buf()))
    }
}
