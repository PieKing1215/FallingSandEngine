use sdl2::pixels::Color;
use crate::game::common::world::material::{PhysicsType, TEST_MATERIAL};
use crate::game::common::world::ChunkHandlerGeneric;
use sdl2::{event::Event, keyboard::Keycode};
use specs::{Builder, Entities, WorldExt, WriteStorage};

use crate::game::common::world::{Position, Velocity, World, entity::{CollisionDetector, GameEntity, Hitbox, PhysicsEntity, Player, PlayerGrappleState, PlayerJumpState, PlayerLaunchState, PlayerMovementMode}, material::MaterialInstance};

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
                launch: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::LShift, KeyControlMode::Momentary)),
                    Box::new(KeyControl::new(Keycode::X, KeyControlMode::Momentary)),
                ])),
                grapple: Box::new(MultiControl::new(MultiControlMode::OR, vec![
                    Box::new(KeyControl::new(Keycode::Z, KeyControlMode::Momentary)),
                ])) 
            },
            camera_scale: 2.0,
            mouse_joint: None,
            main_menu: MainMenu { state: super::ui::MainMenuState::Main, action_queue: Vec::new() }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn tick(&mut self, world: &mut World<ClientChunk>) {
        let mut pixels_to_highlight: Vec<(i64, i64)> = Vec::new();
        if let Some(w) = &mut self.world {
            w.tick(world);

            if let Some(eid) = w.local_entity {
                let (
                    mut entities,
                    mut player,
                    mut game_ent_storage,
                    mut phys_ent_storage,
                    mut velocity_storage,
                    mut position_storage,
                    mut hitbox_storage,
                    mut collision_storage,
                ) = world.ecs.system_data::<(
                    Entities,
                    WriteStorage<Player>,
                    WriteStorage<GameEntity>,
                    WriteStorage<PhysicsEntity>,
                    WriteStorage<Velocity>,
                    WriteStorage<Position>,
                    WriteStorage<Hitbox>,
                    WriteStorage<CollisionDetector>,
                )>();

                let player = player.get_mut(eid).expect("Missing Player component on local_entity");
                
                match player.movement {
                    PlayerMovementMode::Normal { ref mut state, ref mut coyote_time, ref mut boost, ref mut launch_state, ref mut grapple_state } => {
                        if velocity_storage.get_mut(eid).is_some() {
                            // log::debug!("{}", *launch_state);

                            let mut do_normal_movement = true;
                            let mut gravity = true;

                            match launch_state {
                                PlayerLaunchState::Ready => {
                                    if self.controls.launch.get() {
                                        *launch_state = PlayerLaunchState::Hold;
                                    }
                                },
                                PlayerLaunchState::Hold => {
                                    do_normal_movement = false;
                                    gravity = false;
                                    velocity_storage.get_mut(eid).unwrap().x *= 0.75;
                                    velocity_storage.get_mut(eid).unwrap().y *= 0.75;

                                    if !self.controls.launch.get() {
                                        let target_x: f64 = 
                                            if self.controls.left.get() { -10.0 } else { 0.0 } + 
                                            if self.controls.right.get() { 10.0 } else { 0.0 };
                                        let target_y: f64 = 
                                            if self.controls.up.get() { -10.0 } else { 0.0 } + 
                                            if self.controls.down.get() { 10.0 } else { 0.0 };

                                        *launch_state = PlayerLaunchState::Launch {
                                            time: 10, 
                                            dir_x: target_x, 
                                            dir_y: target_y,
                                         };
                                    }
                                },
                                PlayerLaunchState::Launch { time, dir_x, dir_y } => {
                                    do_normal_movement = false;
                                    gravity = false;
                                    if *time == 0 {
                                        *launch_state = PlayerLaunchState::Used;
                                    }else {
                                        *time -= 1;

                                        let target_x: f64 = 
                                            if self.controls.left.get() { -10.0 } else { 0.0 } + 
                                            if self.controls.right.get() { 10.0 } else { 0.0 };
                                        let target_y: f64 = 
                                            if self.controls.up.get() { -10.0 } else { 0.0 } + 
                                            if self.controls.down.get() { 10.0 } else { 0.0 };

                                        *dir_x += (target_x - *dir_x) * 0.05;
                                        *dir_y += (target_y - *dir_y) * 0.05;

                                        velocity_storage.get_mut(eid).unwrap().x = *dir_x;
                                        velocity_storage.get_mut(eid).unwrap().y = *dir_y;
                                    }
                                },
                                PlayerLaunchState::Used => {
                                    if phys_ent_storage.get_mut(eid).expect("Missing PhysicsEntity component on local_entity").on_ground {
                                        *launch_state = PlayerLaunchState::Ready;
                                    }
                                }
                            }

                            match grapple_state {
                                PlayerGrappleState::Ready => {
                                    if self.controls.grapple.get() {
                                        let target_x: f64 = 
                                            if self.controls.left.get() { -16.0 } else { 0.0 } + 
                                            if self.controls.right.get() { 16.0 } else { 0.0 };
                                        let target_y: f64 = 
                                            if self.controls.up.get() { -16.0 } else { 0.0 } + 
                                            if self.controls.down.get() { 16.0 } else { 0.0 };

                                        if target_x != 0.0 || target_y != 0.0 {
                                            let entity = entities.build_entity()
                                                .with(Position{ x: position_storage.get(eid).unwrap().x + target_x, y: position_storage.get(eid).unwrap().y + target_y }, &mut position_storage)
                                                .with(Velocity{ x: velocity_storage.get_mut(eid).unwrap().x * 0.5 + target_x, y: velocity_storage.get_mut(eid).unwrap().y * 0.5 + target_y }, &mut velocity_storage)
                                                .with(Hitbox { x1: -4.0, y1: -4.0, x2: 4.0, y2: 4.0 }, &mut hitbox_storage)
                                                .with(PhysicsEntity { gravity: 0.0, on_ground: false, edge_clip_distance: 0.0, collision: true, collide_with_sand: false }, &mut phys_ent_storage)
                                                .with(GameEntity, &mut game_ent_storage)
                                                .with(CollisionDetector{ collided: false }, &mut collision_storage)
                                                .build();
                                                
                                            *grapple_state = PlayerGrappleState::Out { entity, can_cancel: false, tether_length: 0.0, desired_tether_length: 0.0, pivots: Vec::new() };
                                        }
                                    }
                                },
                                PlayerGrappleState::Out { entity, can_cancel, tether_length, desired_tether_length, pivots } => {
                                    // log::trace!("{:?}", collision_storage.get_mut(*entity));
                                    if let Some(col) = collision_storage.get_mut(*entity) {
                                        let dx = pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).x - position_storage.get(eid).unwrap().x;
                                        let dy = pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).y - position_storage.get(eid).unwrap().y;
                                        let mag = (dx * dx + dy * dy).sqrt();

                                        let raycast_filter = |_pos: (i64, i64), mat: &MaterialInstance| {
                                            mat.physics == PhysicsType::Solid
                                        };

                                        if let Some(r) = world.raycast(
                                            position_storage.get(eid).unwrap().x as i64, position_storage.get(eid).unwrap().y as i64, 
                                            pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).x as i64, pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).y as i64,
                                            raycast_filter)
                                             {
                                            // log::debug!("{} {} => {:?}", r.0.0, r.0.1, r.1);
                                            pixels_to_highlight.push(r.0);

                                            let side_1 = world.chunk_handler.get(r.0.0 + ((dy / mag) * 2.0) as i64, r.0.1 + ((-dx / mag) * 2.0) as i64);
                                            // let side_2 = world.chunk_handler.get(r.0.0 + ((-dy / mag) * 1.0) as i64, r.0.1 + ((dx / mag) * 1.0) as i64);

                                            if side_1.is_ok() && side_1.unwrap().physics != PhysicsType::Air {
                                                pivots.push(Position { x: r.0.0 as f64 + (-dy / mag) * 2.0, y: r.0.1 as f64 + (dx / mag) * 2.0});
                                            } else {
                                                pivots.push(Position { x: r.0.0 as f64 + (dy / mag) * 2.0, y: r.0.1 as f64 + (-dx / mag) * 2.0});
                                            }
                                        }

                                        #[allow(clippy::collapsible_if)]
                                        if pivots.len() > 1 {
                                            if world.raycast(
                                                position_storage.get(eid).unwrap().x as i64, position_storage.get(eid).unwrap().y as i64, 
                                                pivots[pivots.len() - 2].x as i64, pivots[pivots.len() - 2].y as i64,
                                                raycast_filter).is_none() {
                                                pivots.pop();
                                            }
                                        }else if !pivots.is_empty() {
                                            if world.raycast(
                                                position_storage.get(eid).unwrap().x as i64, position_storage.get(eid).unwrap().y as i64, 
                                                position_storage.get(*entity).unwrap().x as i64, position_storage.get(*entity).unwrap().y as i64,
                                                raycast_filter).is_none() {
                                                pivots.pop();
                                            }
                                        }
                                        
                                        if col.collided {

                                            if *desired_tether_length == 0.0 {
                                                *desired_tether_length = mag - 10.0;
                                                *tether_length = mag;

                                                // pivots.push(Position { x: position_storage.get(eid).unwrap().x + dx * 0.75, y: position_storage.get(eid).unwrap().y + dy * 0.75 });
                                                // pivots.push(Position { x: position_storage.get(eid).unwrap().x + dx * 0.5, y: position_storage.get(eid).unwrap().y + dy * 0.5 });
                                            } else {
                                                *tether_length += (*desired_tether_length - *tether_length) * 0.1;
                                            }

                                            if !self.controls.jump.get() {
                                                *can_cancel = true;
                                            }

                                            do_normal_movement = false;
                                            // gravity = false;

                                            // velocity_storage.remove(*entity);
                                            velocity_storage.get_mut(*entity).unwrap().x = 0.0;
                                            velocity_storage.get_mut(*entity).unwrap().y = 0.0;
                                            
                                            /*if mag < 24.0 {
                                                velocity_storage.get_mut(eid).unwrap().x *= 0.6;
                                                velocity_storage.get_mut(eid).unwrap().y *= 0.6;
                                                velocity_storage.get_mut(eid).unwrap().y -= 1.0;

                                                entities.delete(*entity).expect("Failed to queue entity for deletion");
                                                *grapple_state = PlayerGrappleState::Used;
                                            } else */
                                            if self.controls.jump.get() && *can_cancel {
                                                velocity_storage.get_mut(eid).unwrap().x *= 1.4;
                                                velocity_storage.get_mut(eid).unwrap().y *= 1.4;
                                                velocity_storage.get_mut(eid).unwrap().y -= 8.0;

                                                *grapple_state = PlayerGrappleState::Cancelled{ entity: *entity };
                                            }else{

                                                let target_x: f64 = 
                                                    if self.controls.left.get() { -0.1 } else { 0.0 } + 
                                                    if self.controls.right.get() { 0.1 } else { 0.0 };
                                                velocity_storage.get_mut(eid).unwrap().x += target_x;

                                                if self.controls.grapple.get() {
                                                    *desired_tether_length = (*desired_tether_length - 8.0).max(14.0);
                                                }
                                                
                                                let mut remaining_tether = *tether_length;

                                                if pivots.len() > 1 {
                                                    for i in 1..pivots.len() {
                                                        let xx = pivots[i].x - pivots[i-1].x;
                                                        let yy = pivots[i].y - pivots[i-1].y;
                                                        remaining_tether -= (xx * xx + yy * yy).sqrt();
                                                    }
                                                }

                                                if !pivots.is_empty() {
                                                    let xx = position_storage.get(*entity).unwrap().x - pivots.first().unwrap().x;
                                                    let yy = position_storage.get(*entity).unwrap().y - pivots.first().unwrap().y;
                                                    remaining_tether -= (xx * xx + yy * yy).sqrt();
                                                }

                                                // log::debug!("{}", remaining_tether);

                                                if mag > remaining_tether {
                                                    let dx = dx / mag;
                                                    let dy = dy / mag;
    
                                                    let old_pos = position_storage.get_mut(eid).unwrap().clone();
    
                                                    position_storage.get_mut(eid).unwrap().x += ((pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).x - dx * remaining_tether) - position_storage.get_mut(eid).unwrap().x) * 0.25;
                                                    position_storage.get_mut(eid).unwrap().y += ((pivots.last().unwrap_or_else(|| position_storage.get(*entity).unwrap()).y - dy * remaining_tether) - position_storage.get_mut(eid).unwrap().y) * 0.25;
    
                                                    velocity_storage.get_mut(eid).unwrap().x += ((position_storage.get_mut(eid).unwrap().x - (old_pos.x - velocity_storage.get_mut(eid).unwrap().x)) - velocity_storage.get_mut(eid).unwrap().x) * 0.25;
                                                    velocity_storage.get_mut(eid).unwrap().y += ((position_storage.get_mut(eid).unwrap().y - (old_pos.y - velocity_storage.get_mut(eid).unwrap().y)) - velocity_storage.get_mut(eid).unwrap().y) * 0.25;
                                                }
                                            }
                                        }else if mag > 256.0 {
                                            velocity_storage.get_mut(*entity).unwrap().x *= 0.5;
                                            velocity_storage.get_mut(*entity).unwrap().y *= 0.5;
                                            *grapple_state = PlayerGrappleState::Cancelled{ entity: *entity };
                                        }
                                    }
                                },
                                PlayerGrappleState::Cancelled { entity } => {
                                    let dx = position_storage.get(eid).unwrap().x - position_storage.get(*entity).unwrap().x;
                                    let dy = position_storage.get(eid).unwrap().y - position_storage.get(*entity).unwrap().y;
                                    let mag = (dx * dx + dy * dy).sqrt();

                                    phys_ent_storage.get_mut(*entity).unwrap().collision = false;

                                    if mag < 16.0 {
                                        entities.delete(*entity).expect("Failed to queue entity for deletion");
                                        *grapple_state = PlayerGrappleState::Ready; // change this to Used if we want to wait until they hit the ground
                                    } else {
                                        let dx_n = dx / mag;
                                        let dy_n = dy / mag;

                                        if mag < 64.0 {
                                            velocity_storage.get_mut(*entity).unwrap().x += ((dx_n * 20.0) - velocity_storage.get_mut(*entity).unwrap().x) * 0.7;
                                            velocity_storage.get_mut(*entity).unwrap().y += ((dy_n * 20.0) - velocity_storage.get_mut(*entity).unwrap().y) * 0.7;
                                        } else if mag < 80.0 {
                                            velocity_storage.get_mut(*entity).unwrap().x += ((dx_n * 20.0) - velocity_storage.get_mut(*entity).unwrap().x) * 0.4;
                                            velocity_storage.get_mut(*entity).unwrap().y += ((dy_n * 20.0) - velocity_storage.get_mut(*entity).unwrap().y) * 0.4;
                                        } else {
                                            velocity_storage.get_mut(*entity).unwrap().x += ((dx_n * 40.0) - velocity_storage.get_mut(*entity).unwrap().x) * 0.1;
                                            velocity_storage.get_mut(*entity).unwrap().y += ((dy_n * 40.0) - velocity_storage.get_mut(*entity).unwrap().y) * 0.1;
                                        }

                                    }
                                },
                                PlayerGrappleState::Used => {
                                    if phys_ent_storage.get_mut(eid).expect("Missing PhysicsEntity component on local_entity").on_ground {
                                        *grapple_state = PlayerGrappleState::Ready;
                                    }
                                },
                            }

                            let phys_ent = phys_ent_storage.get_mut(eid).expect("Missing PhysicsEntity component on local_entity");
                            if gravity {
                                phys_ent.gravity = 0.5;
                            } else {
                                phys_ent.gravity = 0.0;
                            }

                            if do_normal_movement {
                                let mut target_x: f64 = 
                                    if self.controls.left.get() { -7.0 } else { 0.0 } + 
                                    if self.controls.right.get() { 7.0 } else { 0.0 };
                                let mut inv_accel_x = if phys_ent.on_ground { 6.0 } else { 12.0 };

                                if phys_ent.on_ground {
                                    // *boost = 1.0;
                                    *boost = 0.0;
                                } else {
                                    velocity_storage.get_mut(eid).unwrap().x *= 0.99;
                                    velocity_storage.get_mut(eid).unwrap().y *= 0.99;
                                }

                                if phys_ent.on_ground {
                                    *coyote_time = 6;
                                } else if *coyote_time > 0 {
                                    *coyote_time -= 1;
                                }

                                if self.controls.jump.get() && *coyote_time > 0 && *state == PlayerJumpState::None {
                                    velocity_storage.get_mut(eid).unwrap().y -= 10.0;
                                    target_x *= 1.5;
                                    inv_accel_x *= 0.5;
                                    *coyote_time = 0; // prevent double jumping by quickly spamming

                                    *state = PlayerJumpState::Jumping;
                                }

                                // if self.controls.up.get()    { velocity_storage.get_mut(eid).unwrap().y -= 0.5 }
                                #[allow(clippy::collapsible_if)]
                                if *state == PlayerJumpState::None {
                                    if self.controls.jump.get() && !phys_ent.on_ground && *boost > 0.0 {
                                        velocity_storage.get_mut(eid).unwrap().y -= 0.7;
                                        *boost -= 0.05;
                                    }
                                }else if *state == PlayerJumpState::Jumping {
                                    if !self.controls.jump.get() {
                                        if !phys_ent.on_ground && velocity_storage.get_mut(eid).unwrap().y < 0.0 { 
                                            velocity_storage.get_mut(eid).unwrap().y *= 0.8;
                                        }
                                        *state = PlayerJumpState::None;
                                    }
                                }

                                if self.controls.down.get() {
                                    velocity_storage.get_mut(eid).unwrap().y += 0.1;
                                }

                                if phys_ent.on_ground && velocity_storage.get_mut(eid).unwrap().x.abs() >= 0.001 && target_x.abs() >= 0.001 && target_x.signum() != velocity_storage.get_mut(eid).unwrap().x.signum() {
                                    inv_accel_x *= 0.5;
                                }

                                if target_x.abs() > 0.0 {
                                    velocity_storage.get_mut(eid).unwrap().x += (target_x - velocity_storage.get_mut(eid).unwrap().x) / inv_accel_x;
                                }
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
                            player.movement = PlayerMovementMode::Normal { 
                                state: PlayerJumpState::None,
                                coyote_time: 0,
                                boost: 1.0, 
                                launch_state: PlayerLaunchState::Ready, 
                                grapple_state: PlayerGrappleState::Ready,
                            };
                        }
                    },
                }
            }
            
            world.ecs.maintain();
        }

        // for (x, y) in pixels_to_highlight.iter() {
        //     world.chunk_handler.set(*x, *y, MaterialInstance {
        //         material_id: TEST_MATERIAL.id,
        //         physics: crate::game::common::world::material::PhysicsType::Solid,
        //         color: Color::RGB(255, 255, 255),
        //     });
        //     // world.chunk_handler.set(*x, *y, MaterialInstance::air());
        // }
    }

    pub fn on_event(&mut self, event: &Event) -> bool {
        self.controls.process(&InputEvent::SDL2Event(event));
        false
    }
}