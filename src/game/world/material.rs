use sdl2::pixels::Color;


pub enum PhysicsType {
    Solid,
    Liquid,
    Gas,
}

pub struct Material {
    name: String,
    id: u16,
}

pub struct MaterialInstance {
    material_id: u16,
    physics: PhysicsType,
    color: Color
}

impl MaterialInstance {
    pub fn new(material_id: u16, physics: PhysicsType, color: Color) -> Self {
        Self {
            material_id,
            physics,
            color,
        }
    }
}