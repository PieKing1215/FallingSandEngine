use sdl2::{event::Event, keyboard::Keycode};
use specs::WriteStorage;

use crate::game::common::world::{Velocity, World};

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
                    mut velocity_storage,
                ) = world.ecs.system_data::<(
                    WriteStorage<Velocity>,
                )>();

                if let Some(vel) = velocity_storage.get_mut(eid) {
                    if self.controls.up.get()    { vel.y -= 0.5 }
                    if self.controls.down.get()  { vel.y += 0.5 }
                    if self.controls.left.get()  { vel.x -= 0.5 }
                    if self.controls.right.get() { vel.x += 0.5 }
                }
            }
        }
    }

    pub fn on_event(&mut self, event: &Event) -> bool {
        self.controls.process(&InputEvent::SDL2Event(event));
        false
    }
}