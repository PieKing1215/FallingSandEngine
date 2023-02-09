use rapier2d::{
    na::Isometry2,
    prelude::{ColliderBuilder, InteractionGroups, RigidBodyBuilder},
};
use serde::{Deserialize, Serialize};
use specs::{storage::BTreeStorage, Builder, Component, Entity, WorldExt};

use crate::game::common::world::{
    copy_paste::MaterialBuf, physics::PHYSICS_SCALE, Chunk, CollisionFlags, Loader, Position,
    RigidBodyComponent, Velocity, World,
};

use super::{GameEntity, Hitbox, Persistent, PhysicsEntity};

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

impl PlayerMovementMode {
    pub fn default_normal() -> Self {
        Self::Normal {
            state: PlayerJumpState::None,
            coyote_time: 0,
            boost: 1.0,
            launch_state: PlayerLaunchState::Ready,
            grapple_state: PlayerGrappleState::Ready,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerClipboard {
    pub clipboard: Option<MaterialBuf>,
    pub state: PlayerClipboardState,
}

impl PlayerClipboard {
    pub fn clear(&mut self) {
        self.clipboard = None;
    }
}

impl Default for PlayerClipboard {
    fn default() -> Self {
        Self { clipboard: None, state: PlayerClipboardState::Idle }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutCopy {
    Copy,
    Cut,
}

#[derive(Debug, Clone)]
pub enum PlayerClipboardState {
    Idle,

    // before the player started dragging
    PreSelecting(CutCopy),

    // player currently dragging
    Selecting(CutCopy, Position),

    Pasting,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub movement: PlayerMovementMode,
    pub clipboard: PlayerClipboard,
}

impl Player {
    pub fn create_and_add<C: Chunk>(world: &mut World<C>) -> Entity {
        let rigid_body = RigidBodyBuilder::dynamic()
            .position(Isometry2::new([0.0, 20.0].into(), 0.0))
            .lock_rotations()
            .gravity_scale(0.0)
            .build();
        let handle = world.physics.bodies.insert(rigid_body);
        let collider =
            ColliderBuilder::cuboid(12.0 / PHYSICS_SCALE / 2.0, 20.0 / PHYSICS_SCALE / 2.0)
                .collision_groups(InteractionGroups::new(
                    CollisionFlags::PLAYER.bits().into(),
                    (CollisionFlags::RIGIDBODY | CollisionFlags::ENTITY)
                        .bits()
                        .into(),
                ))
                .density(1.5)
                .friction(0.3)
                .build();
        let _co_handle =
            world
                .physics
                .colliders
                .insert_with_parent(collider, handle, &mut world.physics.bodies);

        let player = world
            .ecs
            .create_entity()
            .with(Player {
                movement: PlayerMovementMode::default_normal(),
                clipboard: PlayerClipboard::default(),
            })
            .with(GameEntity)
            .with(PhysicsEntity {
                on_ground: false,
                gravity: 0.5,
                edge_clip_distance: 2.0,
                collision: true,
                collide_with_sand: true,
            })
            .with(Persistent)
            .with(Position { x: 0.0, y: -20.0 })
            .with(Velocity { x: 0.0, y: 0.0 })
            .with(Hitbox { x1: -6.0, y1: -10.0, x2: 6.0, y2: 10.0 })
            .with(Loader)
            .with(RigidBodyComponent::of(handle))
            .build();

        player
    }
}

impl Component for Player {
    type Storage = BTreeStorage<Self>;
}
