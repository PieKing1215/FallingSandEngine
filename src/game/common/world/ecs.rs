use core::fmt::Debug;
use std::sync::{Arc, Mutex};

use specs::{Component, NullStorage, VecStorage};
use serde::{Serialize, Deserialize};

use super::{Chunk, ChunkHandler, ChunkHandlerGeneric, gen::WorldGenerator};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
}

impl Component for Velocity {
    type Storage = VecStorage<Self>;
}

pub struct ChunkHandlerResource<'a>(pub &'a mut (dyn ChunkHandlerGeneric));

impl Debug for ChunkHandlerResource<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ChunkHandlerGeneric")
    }
}

pub struct FilePersistent;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Loader;

impl Component for Loader {
    type Storage = NullStorage<Self>;
}