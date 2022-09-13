use serde::{Deserialize, Serialize};
use specs::{storage::BTreeStorage, Component, Entity};

use crate::game::common::world::Position;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum PlayerJumpState {
    None,
    Jumping,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlayerLaunchState {
    Ready,
    Hold,
    Launch { time: u16, dir_x: f64, dir_y: f64 },
    Used,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlayerGrappleState {
    Ready,
    Out {
        can_cancel: bool,
        entity: Entity,
        tether_length: f64,
        desired_tether_length: f64,
        pivots: Vec<Position>,
    },
    Cancelled {
        entity: Entity,
    },
    Used,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlayerMovementMode {
    Normal {
        state: PlayerJumpState,
        coyote_time: u8,
        boost: f32,
        launch_state: PlayerLaunchState,
        grapple_state: PlayerGrappleState,
    },
    Free,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub movement: PlayerMovementMode,
}

impl Component for Player {
    type Storage = BTreeStorage<Self>;
}
