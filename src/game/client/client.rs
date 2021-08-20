use sdl2::{event::Event, keyboard::Keycode};
use specs::WriteStorage;

use crate::game::common::world::{Position, World};

use super::{ui::MainMenu, world::{ClientChunk, ClientWorld}};

// TODO: actually implement this properly with functions and stuff instead of just raw field accesses
pub struct Controls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

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
                up: false,
                down: false,
                left: false,
                right: false,
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
                    mut position_storage,
                ) = world.ecs.system_data::<(
                    WriteStorage<Position>,
                )>();

                if let Some(pos) = position_storage.get_mut(eid) {
                    if self.controls.up    { pos.y -= 4.0 }
                    if self.controls.down  { pos.y += 4.0 }
                    if self.controls.left  { pos.x -= 4.0 }
                    if self.controls.right { pos.x += 4.0 }
                }
            }
        }
    }

    pub fn on_event(&mut self, event: &Event) -> bool {

        match event {
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                self.controls.up = true;
                return true;
            },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                self.controls.left = true;
                return true;
            },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                self.controls.down = true;
                return true;
            },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                self.controls.right = true;
                return true;
            },
            Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                self.controls.up = false;
                return true;
            },
            Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                self.controls.left = false;
                return true;
            },
            Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                self.controls.down = false;
                return true;
            },
            Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                self.controls.right = false;
                return true;
            },
            _ => {},
        }

        false
    }
}