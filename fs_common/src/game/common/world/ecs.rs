use core::fmt::Debug;
use std::ops::Deref;

use bitflags::bitflags;
use rapier2d::{
    na::{Isometry2, Vector2},
    prelude::RigidBodyHandle,
};
use serde::{Deserialize, Serialize};
use specs::{
    storage::{BTreeStorage, MaskedStorage},
    Component, Entities, Join, NullStorage, Read, ReadStorage, Storage, System, VecStorage,
    WriteStorage,
};

use crate::game::common::world::physics::PHYSICS_SCALE;

use super::{entity::Hitbox, physics::Physics, ChunkHandlerGeneric};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Default)]
pub struct TickTime(pub u32);

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Camera;

impl Component for Camera {
    type Storage = NullStorage<Self>;
}

// TODO: try to figure out a good way to make this Serialize/Deserialize
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Target {
    Entity(specs::Entity),
    Position(Position),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TargetStyle {
    Locked,
    Linear(f64),
    EaseOut(f64),
}

// TODO: try to figure out a good way to make this Serialize/Deserialize
#[derive(Debug, Clone)]
pub struct AutoTarget {
    pub target: Target,
    pub offset: (f64, f64),
    pub style: TargetStyle,
}

impl AutoTarget {
    pub fn get_target_pos<S>(&self, pos_storage: &Storage<Position, S>) -> Option<Position>
    where
        S: Deref<Target = MaskedStorage<Position>>,
    {
        match &self.target {
            Target::Entity(e) => pos_storage.get(*e).cloned(),
            Target::Position(p) => Some(p.clone()),
        }
        .map(|p| Position { x: p.x + self.offset.0, y: p.y + self.offset.1 })
    }

    pub fn get_target_vel<S>(&self, vel_storage: &Storage<Velocity, S>) -> Option<Velocity>
    where
        S: Deref<Target = MaskedStorage<Velocity>>,
    {
        match &self.target {
            Target::Entity(e) => vel_storage.get(*e).cloned(),
            Target::Position(_) => None,
        }
    }
}

impl Component for AutoTarget {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdateAutoTargets;

impl<'a> System<'a> for UpdateAutoTargets {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, AutoTarget>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateAutoTargets::run");

        let (entities, delta_time, target, mut pos_storage, mut vel_storage) = data;

        (&entities, &target).join().for_each(|(entity, at)| {
            if let Some(target_pos) = at.get_target_pos(&pos_storage) {
                let pos = pos_storage
                    .get_mut(entity)
                    .expect("AutoTarget missing Position");
                match at.style {
                    TargetStyle::Locked => {
                        pos.x = target_pos.x;
                        pos.y = target_pos.y;
                    },
                    TargetStyle::EaseOut(factor) => {
                        pos.x += (target_pos.x - pos.x)
                            * (factor * delta_time.0.as_secs_f64()).clamp(0.0, 1.0);
                        pos.y += (target_pos.y - pos.y)
                            * (factor * delta_time.0.as_secs_f64()).clamp(0.0, 1.0);
                    },
                    TargetStyle::Linear(speed) => {
                        let dx = target_pos.x - pos.x;
                        let dy = target_pos.y - pos.y;
                        let mag = (dx * dx + dy * dy).sqrt();
                        if mag <= speed * delta_time.0.as_secs_f64() {
                            pos.x = target_pos.x;
                            pos.y = target_pos.y;
                        } else if mag > 0.0 {
                            pos.x += dx / mag * speed * delta_time.0.as_secs_f64();
                            pos.y += dy / mag * speed * delta_time.0.as_secs_f64();
                        }
                    },
                }
            }

            if let Some(target_vel) = at.get_target_vel(&vel_storage) {
                if let Some(vel) = vel_storage.get_mut(entity) {
                    *vel = target_vel;
                }
            }
        });
    }
}

bitflags! {
    pub struct CollisionFlags: u32 {
        const ENTITY    = 0b0000_0000_0000_0001;
        const WORLD     = 0b0000_0000_0000_0010;
        const RIGIDBODY = 0b0000_0000_0000_0100;
        const PLAYER    = Self::ENTITY.bits;
    }
}

#[derive(Debug, Clone)]
pub struct RigidBodyComponent {
    pub body: RigidBodyHandle,
}

impl RigidBodyComponent {
    pub fn of(body: RigidBodyHandle) -> Self {
        Self { body }
    }
}

impl Component for RigidBodyComponent {
    type Storage = BTreeStorage<Self>;
}

pub struct UpdateRigidBodies<'a> {
    pub physics: &'a mut Physics,
}

impl<'a> System<'a> for UpdateRigidBodies<'a> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadStorage<'a, Hitbox>,
        WriteStorage<'a, RigidBodyComponent>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateRigidBodies::run");

        let (hitboxes, mut bodies, mut pos, mut vel) = data;

        (&hitboxes, &mut bodies, &mut pos, &mut vel)
            .join()
            .for_each(|(_hitbox, body, pos, vel)| {
                let body = self.physics.bodies.get_mut(body.body).unwrap();
                let np = Vector2::new(pos.x as f32 / PHYSICS_SCALE, pos.y as f32 / PHYSICS_SCALE);
                body.set_position(Isometry2::new(np, 0.0), true);
                body.set_linvel(Vector2::new(vel.x as f32, vel.y as f32), true);
            });
    }
}

pub struct ApplyRigidBodies<'a> {
    pub physics: &'a mut Physics,
}

impl<'a> System<'a> for ApplyRigidBodies<'a> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadStorage<'a, Hitbox>,
        ReadStorage<'a, RigidBodyComponent>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("ApplyRigidBodies::run");

        let (hitboxes, bodies, mut pos, mut vel) = data;

        (&hitboxes, &bodies, &mut pos, &mut vel)
            .join()
            .for_each(|(_hitbox, body, _pos, vel)| {
                let body = self.physics.bodies.get(body.body).unwrap();

                // TODO: I want to take this into account since rapier will update the position when clipping
                //         but since it also adds the velocity, it causes the player to clip into walls slightly (causing jitter)
                // pos.x = f64::from(body.translation().x * PHYSICS_SCALE);
                // pos.y = f64::from(body.translation().y * PHYSICS_SCALE);

                let vel_before = vel.clone();

                vel.x = f64::from(body.linvel().x);
                vel.y = f64::from(body.linvel().y);

                let vel_change = Velocity { x: vel.x - vel_before.x, y: vel.y - vel_before.y };

                let vel_change_limit = 30.0;
                let vel_change_mag = vel_change.x * vel_change.x + vel_change.y * vel_change.y;

                if vel_change_mag > vel_change_limit / 10.0 {
                    // arbitrary limit to simplify math when vel_change_mag is too small to notice
                    // function that takes x: [0..inf], y: [0..inf] and maps it to x: [0..inf], y: [0..vel_change_limit]
                    // see https://www.desmos.com/calculator/oi3loraack for a comparison of some functions
                    let new_mag = (vel_change_mag / vel_change_limit).tanh() * vel_change_limit;

                    log::debug!(
                        "Limited body velocity change ({} -> {})",
                        vel_change_mag,
                        new_mag
                    );
                    vel.x = vel_before.x + vel_change.x * (new_mag / vel_change_mag);
                    vel.y = vel_before.y + vel_change.y * (new_mag / vel_change_mag);
                }
            });
    }
}
