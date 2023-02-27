use crate::game::common::world::material::MaterialInstance;

use super::MaterialPlacerSampler;

pub struct LitPlacer<T> {
    base: T,
    light: f32,
}

impl<T> LitPlacer<T> {
    pub fn new(base: T, light: f32) -> Self {
        Self { base, light }
    }
}

impl<T: MaterialPlacerSampler> MaterialPlacerSampler for LitPlacer<T> {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance {
        let mut p = self.base.pixel(x, y);
        p.light = self.light;
        p
    }
}

pub trait LitExt {
    fn lit(self, light: f32) -> LitPlacer<Self>
    where
        Self: Sized;
}

impl<T: MaterialPlacerSampler> LitExt for T {
    fn lit(self, light: f32) -> LitPlacer<Self>
    where
        Self: Sized,
    {
        LitPlacer::new(self, light)
    }
}
