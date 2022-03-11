use sdl2::pixels::Color;
use serde::{Deserialize, Serialize};

pub static AIR: Material = Material { id: 0, name: "Air" };

pub static TEST_MATERIAL: Material = Material { id: 1, name: "Test Material" };

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum PhysicsType {
    Air,
    Solid,
    Sand,
    Liquid,
    Gas,
    Object,
}

pub struct Material<'a> {
    pub id: u16,
    pub name: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Color")]
struct ColorDef {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MaterialInstance {
    pub material_id: u16,
    pub physics: PhysicsType,
    #[serde(with = "ColorDef")]
    pub color: Color,
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

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}
