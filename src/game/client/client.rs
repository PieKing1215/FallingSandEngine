use sdl2::{event::Event, keyboard::Keycode};
use specs::WriteStorage;

use crate::game::common::world::{Velocity, World, entity::{PhysicsEntity, Player, PlayerJumpState, PlayerMovementMode}};

use super::{input::{Controls, InputEvent, KeyControl, KeyControlMode, MultiControl, MultiControlMode}, ui::MainMenu, world::{ClientChunk, ClientWorld}};

pub struct Client {
    pub world: Option<ClientWorld>,
    pub controls: Controls,
    pub camera_scale: f64,
    pub mouse_joint: Option<liquidfun::box2d::dynamics::joints::mouse_joint::MouseJoint>,
    pub main_menu: MainMenu,
}

impl Client {
    pub fn new() -> Self {
        Self {
            world: None,
            controls: Controls {
                up: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::W, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::Up, KeyControlMode::Momentary)),
                ])),
                down: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::S, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::Down, KeyControlMode::Momentary)),
                ])),
                left: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::A, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::Left, KeyControlMode::Momentary)),
                ])),
                right: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::D, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::Right, KeyControlMode::Momentary)),
                ])),
                jump: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::Space, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::C, KeyControlMode::Momentary)),
                ])),
                free_fly: Box::new(KeyControl::new(Keycode::Kp1, KeyControlMode::Rising)),
            },
            camera_scale: 2.0,
            mouse_joint: None,
            main_menu: MainMenu { state: super::ui::MainMenuState::Main, action_queue: Vec::new() }
        }
    }

    pub fn tick(&mut self, world: &mut World<ClientChunk>) {
        if let Some(w) = &mut self.world {
            w.tick(world);

            if let Some(eid) = w.local_entity {
                let (
                    mut player,
                    mut phys_ent,
                    mut velocity_storage,
                ) = world.ecs.system_data::<(
                    WriteStorage<Player>,
                    WriteStorage<PhysicsEntity>,
                    WriteStorage<Velocity>,
                )>();

                let player = player.get_mut(eid).expect("Missing Player component on local_entity");
                let phys_ent = phys_ent.get_mut(eid).expect("Missing PhysicsEntity component on local_entity");
                
                match player.movement {
                    PlayerMovementMode::Normal { ref mut state, ref mut boost } => {
                        if let Some(vel) = velocity_storage.get_mut(eid) {

                            let mut target_x: f64 = 
                                if self.controls.left.get() { -7.0 } else { 0.0 } + 
                                if self.controls.right.get() { 7.0 } else { 0.0 };
                            let mut inv_accel_x = if phys_ent.on_ground { 6.0 } else { 12.0 };

                            if phys_ent.on_ground {
                                *boost = 1.0;
                            } else {
                                vel.x *= 0.99;
                                vel.y *= 0.99;
                            }

                            if self.controls.jump.get() && phys_ent.on_ground { 
                                vel.y -= 10.0;
                                target_x *= 2.0;
                                inv_accel_x *= 0.5;

                                *state = PlayerJumpState::Jumping;
                            }

                            // if self.controls.up.get()    { vel.y -= 0.5 }
                            #[allow(clippy::collapsible_if)]
                            if *state == PlayerJumpState::None {
                                if self.controls.jump.get() && !phys_ent.on_ground && *boost > 0.0 {
                                    vel.y -= 0.7;
                                    *boost -= 0.05;
                                }
                            }else if *state == PlayerJumpState::Jumping {
                                if !self.controls.jump.get() {
                                    if !phys_ent.on_ground && vel.y < 0.0 { 
                                        vel.y *= 0.8;
                                    }
                                    *state = PlayerJumpState::None;
                                }
                            }

                            if self.controls.down.get()  { vel.y += 0.1 }


                            if phys_ent.on_ground && vel.x.abs() >= 0.001 && target_x.abs() >= 0.001 && target_x.signum() != vel.x.signum() {
                                inv_accel_x *= 0.5;
                            }

                            if target_x.abs() > 0.0 {
                                vel.x += (target_x - vel.x) / inv_accel_x;
                            }
                        }

                        if self.controls.free_fly.get() {
                            player.movement = PlayerMovementMode::Free;
                        }
                    },
                    PlayerMovementMode::Free => {
                        if let Some(vel) = velocity_storage.get_mut(eid) {
                            if self.controls.up.get()    { vel.y -= 0.7 }
                            if self.controls.down.get()  { vel.y += 0.5 }
                            if self.controls.left.get()  { vel.x -= 0.5 }
                            if self.controls.right.get() { vel.x += 0.5 }
                        }

                        if self.controls.free_fly.get() {
                            player.movement = PlayerMovementMode::Normal { state: PlayerJumpState::None, boost: 1.0 };
                        }
                    },
                }
            }
        }
    }

    pub fn on_event(&mut self, event: &Event) -> bool {
        self.controls.process(&InputEvent::SDL2Event(event));
        false
    }
}