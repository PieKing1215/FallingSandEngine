use specs::{Component, Entities, Join, System, Write, WriteStorage, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

mod player;
pub use player::*;

use super::{ChunkState, ChunkHandlerGeneric, Position, Velocity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntity;

impl Component for GameEntity {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hitbox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Component for Hitbox {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsEntity;

impl Component for PhysicsEntity {
    type Storage = BTreeStorage<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persistent;

impl Component for Persistent {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdatePhysicsEntities<'a>{
    pub chunk_handler: &'a mut (dyn ChunkHandlerGeneric)
}

impl<'a> System<'a> for UpdatePhysicsEntities<'a> {
    #[allow(clippy::type_complexity)]
    type SystemData = (Entities<'a>,
                       WriteStorage<'a, Position>,
                       WriteStorage<'a, Velocity>,
                       WriteStorage<'a, GameEntity>,
                       WriteStorage<'a, PhysicsEntity>,
                       WriteStorage<'a, Persistent>,
                       WriteStorage<'a, Hitbox>);

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdatePhysicsEntities::run");

        let (entities, mut pos, mut vel, mut game_ent, mut phys_ent, mut persistent, mut hitbox) = data;
        // let chunk_handler = chunk_handler.unwrap().0;
        let chunk_handler = &mut *self.chunk_handler;

        // TODO: if I can ever get ChunkHandler to be Send (+ Sync would be ideal), can use par_join and organize a bit for big performance gain
        //       iirc right now, ChunkHandler<ServerChunk> is Send + !Sync and ChunkHandler<ClientChunk> is !Send + !Sync (because of the GPUImage in ChunkGraphics)
        (&entities, &mut pos, &mut vel, &mut game_ent, &mut phys_ent, persistent.maybe(), &mut hitbox).join().for_each(|(ent, pos, vel, game_ent, phys_ent, persistent, hitbox)| {
            // profiling::scope!("Particle");

            let lx = pos.x;
            let ly = pos.y;

            let (chunk_x, chunk_y) = chunk_handler.pixel_to_chunk_pos(lx as i64, ly as i64);
            // skip if chunk not active
            if persistent.is_none() && !matches!(chunk_handler.get_chunk(chunk_x, chunk_y), Some(c) if c.get_state() == ChunkState::Active) {
                return;
            }

            vel.y += 0.1;

            let dx = vel.x;
            let dy = vel.y;

            let steps = (dx.abs() + dy.abs()).sqrt() as u32 + 1;
            for s in 0..steps {
                // profiling::scope!("step");
                let thru = f64::from(s + 1) / f64::from(steps);

                pos.x = lx + dx * thru;
                pos.y = ly + dy * thru;

                if let Ok(mat) = chunk_handler.get(pos.x as i64, pos.y as i64) {
                    
                }
            }
        });
    }
}