use crate::game::common::world::material::MaterialInstance;

use super::MaterialPlacerSampler;

pub struct LitColoredPlacer<T> {
    base: T,
    strength: f32,
}

impl<T> LitColoredPlacer<T> {
    pub fn new(base: T, strength: f32) -> Self {
        Self { base, strength }
    }
}

impl<T: MaterialPlacerSampler> MaterialPlacerSampler for LitColoredPlacer<T> {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance {
        let mut p = self.base.pixel(x, y);
        p.light = [
            p.color.r_f32() * self.strength,
            p.color.g_f32() * self.strength,
            p.color.b_f32() * self.strength,
        ];
        p
    }
}

pub trait LitColoredExt {
    fn lit_colored(self, strength: f32) -> LitColoredPlacer<Self>
    where
        Self: Sized;
}

impl<T: MaterialPlacerSampler> LitColoredExt for T {
    fn lit_colored(self, strength: f32) -> LitColoredPlacer<Self>
    where
        Self: Sized,
    {
        LitColoredPlacer::new(self, strength)
    }
}
