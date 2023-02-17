pub mod test;

use crate::game::common::{world::material::MaterialInstance, Registries};

pub type BiomeID = &'static str;

pub trait Biome {
    fn pixel(&self, x: i64, y: i64, registries: &Registries) -> MaterialInstance;
}

pub struct BiomePlacement {
    pub points: Vec<(BiomePlacementParameter, Box<dyn Biome + Send + Sync>)>,
}

#[derive(Debug, Clone, Copy)]
pub struct BiomePlacementParameter {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

impl BiomePlacementParameter {
    pub fn dist_sq(&self, other: &BiomePlacementParameter) -> f32 {
        let da = self.a - other.a;
        let db = self.b - other.b;
        let dc = self.c - other.c;
        da * da + db * db + dc * dc
    }
}

impl BiomePlacement {
    pub fn nearest(&self, test: BiomePlacementParameter) -> &dyn Biome {
        self.points
            .iter()
            .min_by(|a, b| test.dist_sq(&a.0).partial_cmp(&test.dist_sq(&b.0)).unwrap())
            .unwrap()
            .1
            .as_ref()
    }
}
