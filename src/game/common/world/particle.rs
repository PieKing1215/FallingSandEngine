use super::material::MaterialInstance;


pub struct Particle {
    pub material: MaterialInstance,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}