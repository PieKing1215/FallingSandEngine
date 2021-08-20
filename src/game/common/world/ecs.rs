use std::ops::Deref;
use core::fmt::Debug;
use std::sync::{Arc, Mutex};

use specs::{Component, Entities, Join, NullStorage, Read, ReadStorage, Storage, System, VecStorage, WriteStorage, storage::{BTreeStorage, MaskedStorage}};
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

#[derive(Default)]
pub struct DeltaTime(pub std::time::Duration);

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

// TODO: try to figure out a good way to make this Serialize/Deserialize
#[derive(Debug, Clone)]
pub enum Target {
    Entity(specs::Entity),
    Position(Position),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Camera;

impl Component for Camera {
    type Storage = NullStorage<Self>;
}

// TODO: try to figure out a good way to make this Serialize/Deserialize
#[derive(Debug, Clone)]
pub struct AutoTarget {
    pub target: Target,
    pub offset: (f64, f64),
}

impl AutoTarget {
    pub fn get_target_pos<S>(&self, pos_storage: &Storage<Position, S>) -> Option<Position> 
where S: Deref<Target = MaskedStorage<Position>> {
        match &self.target {
            Target::Entity(e) => pos_storage.get(*e).cloned(),
            Target::Position(p) => Some(p.clone()),
        }
    }
}

impl Component for AutoTarget {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdateAutoTargets;

impl<'a> System<'a> for UpdateAutoTargets {
    #[allow(clippy::type_complexity)]
    type SystemData = (Entities<'a>,
                       Read<'a, DeltaTime>,
                       ReadStorage<'a, AutoTarget>,
                       WriteStorage<'a, Position>);

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateAutoTargets::run");

        let (entities, delta_time, target, mut pos_storage) = data;

        (&entities, &target).join().for_each(|(entity, at)| {
            if let Some(target_pos) = at.get_target_pos(&pos_storage) {
                let pos = pos_storage.get_mut(entity).expect("AutoTarget missing Position");
                pos.x += (target_pos.x - pos.x) * (0.5 * delta_time.0.as_secs_f64()).clamp(0.0, 1.0);
                pos.y += (target_pos.y - pos.y) * (0.5 * delta_time.0.as_secs_f64()).clamp(0.0, 1.0);
            }
        });

    }
}