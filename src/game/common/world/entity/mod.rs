use specs::{Component, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

mod player;
pub use player::*;

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