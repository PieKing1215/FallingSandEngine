use core::fmt::Debug;
use std::sync::{Arc, Mutex};

use specs::{Component, VecStorage};

use super::{Chunk, ChunkHandler, ChunkHandlerGeneric, gen::WorldGenerator};

#[derive(Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
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