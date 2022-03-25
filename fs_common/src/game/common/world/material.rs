use serde::{Deserialize, Serialize};

pub static AIR: Material = Material { id: 0, name: "Air" };

pub static TEST_MATERIAL: Material = Material { id: 1, name: "Test Material" };

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub trait IntoColorNum {
    fn col_num(self) -> u8;
}

impl IntoColorNum for u8 {
    #[inline]
    fn col_num(self) -> u8 {
        self
    }
}

impl IntoColorNum for f32 {
    #[inline]
    fn col_num(self) -> u8 {
        (self * f32::from(u8::MAX)).clamp(f32::from(u8::MIN), f32::from(u8::MAX)) as u8
    }
}

impl Color {
    #[inline]
    pub const fn rgba_const(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb_const(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_const(r, g, b, u8::MAX)
    }

    #[inline]
    pub fn rgba(
        r: impl IntoColorNum,
        g: impl IntoColorNum,
        b: impl IntoColorNum,
        a: impl IntoColorNum,
    ) -> Self {
        Self::rgba_const(r.col_num(), g.col_num(), b.col_num(), a.col_num())
    }

    #[inline]
    pub fn rgb(r: impl IntoColorNum, g: impl IntoColorNum, b: impl IntoColorNum) -> Self {
        Self::rgba(r, g, b, u8::MAX)
    }

    #[inline]
    #[must_use]
    pub fn with_a(self, a: impl IntoColorNum) -> Self {
        Self::rgba_const(self.r, self.g, self.b, a.col_num())
    }

    pub fn r_f32(&self) -> f32 {
        f32::from(self.r) / f32::from(u8::MAX)
    }

    pub fn g_f32(&self) -> f32 {
        f32::from(self.g) / f32::from(u8::MAX)
    }

    pub fn b_f32(&self) -> f32 {
        f32::from(self.b) / f32::from(u8::MAX)
    }

    pub fn a_f32(&self) -> f32 {
        f32::from(self.a) / f32::from(u8::MAX)
    }

    pub const BLACK: Color = Color::rgb_const(0, 0, 0);
    pub const WHITE: Color = Color::rgb_const(0xff, 0xff, 0xff);
    pub const GRAY: Color = Color::rgb_const(0x7f, 0x7f, 0x7f);

    pub const RED: Color = Color::rgb_const(0xff, 0, 0);
    pub const GREEN: Color = Color::rgb_const(0, 0xff, 0);
    pub const BLUE: Color = Color::rgb_const(0, 0, 0xff);

    pub const YELLOW: Color = Color::rgb_const(0xff, 0xff, 0);
    pub const CYAN: Color = Color::rgb_const(0, 0xff, 0xff);
    pub const MAGENTA: Color = Color::rgb_const(0xff, 0, 0xff);

    pub const ORANGE: Color = Color::rgb_const(0xff, 0x7f, 0);
    pub const CHARTREUSE_GREEN: Color = Color::rgb_const(0x7f, 0xff, 0);
    pub const ROSE: Color = Color::rgb_const(0xff, 0, 0x7f);
    pub const VIOLET: Color = Color::rgb_const(0x7f, 0, 0xff);
    pub const SPRING_GREEN: Color = Color::rgb_const(0, 0xff, 0x7f);
    pub const AZURE: Color = Color::rgb_const(0, 0x7f, 0xff);
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> [f32; 4] {
        [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()]
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(color: Color) -> (f32, f32, f32, f32) {
        (color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32())
    }
}
impl From<Color> for [f32; 3] {
    fn from(color: Color) -> [f32; 3] {
        [color.r_f32(), color.g_f32(), color.b_f32()]
    }
}

impl From<Color> for (f32, f32, f32) {
    fn from(color: Color) -> (f32, f32, f32) {
        (color.r_f32(), color.g_f32(), color.b_f32())
    }
}

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

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MaterialInstance {
    pub material_id: u16,
    pub physics: PhysicsType,
    pub color: Color,
}

impl MaterialInstance {
    pub fn air() -> Self {
        Self {
            material_id: AIR.id,
            physics: PhysicsType::Air,
            color: Color::rgba(0, 0, 0, 0),
        }
    }
}

impl Default for MaterialInstance {
    fn default() -> Self {
        Self::air()
    }
}
