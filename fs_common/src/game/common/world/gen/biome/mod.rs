pub mod test;

use crate::game::common::world::material::MaterialInstance;

pub trait Biome {
    fn pixel(&self) -> MaterialInstance;
}

pub struct BiomePlacement {
    pub points: Vec<(BiomePlacementParameter, &'static (dyn Biome + Send + Sync))>,
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
    pub fn nearest(&self, test: BiomePlacementParameter) -> &'static dyn Biome {
        self.points
            .iter()
            .min_by(|a, b| test.dist_sq(&a.0).partial_cmp(&test.dist_sq(&b.0)).unwrap())
            .unwrap()
            .1
    }
}