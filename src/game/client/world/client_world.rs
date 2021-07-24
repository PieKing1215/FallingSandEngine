
use crate::game::common::world::{World, material::MaterialInstance};

use super::ClientChunk;

pub struct ClientWorld {
    pub local_entity_id: Option<u32>,
}

impl ClientWorld {
    pub fn tick(&mut self, _world: &mut World<ClientChunk>) {
        
    }
}

impl World<ClientChunk> {
    pub fn sync_chunk(&mut self, chunk_x: i32, chunk_y: i32, pixels: Vec<MaterialInstance>, colors: Vec<u8>) -> Result<(), String> {
        self.chunk_handler.sync_chunk(chunk_x, chunk_y, pixels, colors)
    }
}
