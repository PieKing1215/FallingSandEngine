use super::material::MaterialInstance;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Particle {
    pub material: MaterialInstance,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub in_object_state: InObjectState,
}

impl Particle {
    pub fn new(material: MaterialInstance, x: f32, y: f32, vx: f32, vy: f32) -> Self {
        Self {
            material,
            x, y,
            vx, vy,
            in_object_state: InObjectState::FirstFrame,
        }
    }
}

#[derive(PartialEq, Serialize, Deserialize)]
pub enum InObjectState {
    FirstFrame,
    Inside,
    Outside,
}