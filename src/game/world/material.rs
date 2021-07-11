use sdl2::pixels::Color;

pub static AIR: Material = Material {
    id: 0,
    name: "Air",
};

pub static TEST_MATERIAL: Material = Material {
    id: 1,
    name: "Test Material",
};

#[derive(Clone, Copy, PartialEq)]
pub enum PhysicsType {
    Air,
    Solid,
    Sand,
    Liquid,
    Gas,
}

pub struct Material<'a> {
    pub id: u16,
    pub name: &'a str,
}

#[derive(Clone, Copy)]
pub struct MaterialInstance {
    pub material_id: u16,
    pub physics: PhysicsType,
    pub color: Color
}

impl MaterialInstance {
    pub fn air() -> Self {
        Self {
            material_id: AIR.id,
            physics: PhysicsType::Air,
            color: Color::RGBA(0, 0, 0, 0),
        }
    }
}