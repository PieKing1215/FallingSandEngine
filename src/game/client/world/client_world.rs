
use crate::game::common::world::World;

use super::ClientChunk;

pub struct ClientWorld {
    pub local_entity_id: Option<u32>,
}

impl ClientWorld {
    pub fn tick(&mut self, world: &mut World<ClientChunk>) {
        
    }
}
