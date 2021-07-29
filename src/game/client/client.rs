use sdl2::{event::Event, keyboard::Keycode};

use crate::game::common::world::World;

use super::world::{ClientChunk, ClientWorld};

// TODO: actually implement this properly with functions and stuff instead of just raw field accesses
pub struct Controls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

pub struct Client {
    pub world: Option<ClientWorld>,
    pub controls: Controls,
    pub camera: Camera,
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
            camera: Camera {
                x: 0.0,
                y: 0.0,
                scale: 2.0,
            },
        }
    }

    pub fn tick(&mut self, world: &mut World<ClientChunk>) {
        if let Some(w) = &mut self.world {
            w.tick(world);

            if let Some(eid) = w.local_entity_id {
                if let Some(le) = world.get_entity_mut(eid) {
                    if self.controls.up { le.y -= 4.0 }
                    if self.controls.down { le.y += 4.0 }
                    if self.controls.left { le.x -= 4.0 }
                    if self.controls.right { le.x += 4.0 }
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