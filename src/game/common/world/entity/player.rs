use specs::{Component, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PlayerJumpState {
    None,
    Jumping,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PlayerMovementMode {
    Normal {
        state: PlayerJumpState,
        boost: f32,
    },
    Free,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub movement: PlayerMovementMode,
}

impl Component for Player {
    type Storage = BTreeStorage<Self>;
}
