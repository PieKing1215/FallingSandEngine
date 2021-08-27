use specs::{Component, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PlayerJumpState {
    None,
    Jumping,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PlayerLaunchState {
    Ready,
    Hold,
    Launch {
        time: u16,
        dir_x: f64,
        dir_y: f64,
    },
    Used,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum PlayerMovementMode {
    Normal {
        state: PlayerJumpState,
        boost: f32,
        launch_state: PlayerLaunchState,
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
