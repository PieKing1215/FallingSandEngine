use simdnoise::NoiseBuilder;

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

impl From<[f32; 3]> for BiomePlacementParameter {
    fn from(value: [f32; 3]) -> Self {
        Self { a: value[0], b: value[1], c: value[2] }
    }
}

pub const BIOME_SIZE: u16 = 200;

pub fn single_random_at(x: f32, y: f32, freq: f32, seed: i32) -> f32 {
    NoiseBuilder::gradient_2d_offset(x, 1, y, 1)
        .with_freq(freq)
        .with_seed(seed)
        .generate()
        .0[0]
}

pub fn biome_params_at(x: i64, y: i64, seed: i32) -> BiomePlacementParameter {
    let factor_a =
        (single_random_at(x as f32, y as f32, 0.000_8, seed + 4) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_b =
        (single_random_at(x as f32, y as f32, 0.000_4, seed + 5) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_c =
        (single_random_at(x as f32, y as f32, 0.000_2, seed + 6) * 20.0 + 0.5).clamp(0.0, 1.0);
    BiomePlacementParameter { a: factor_a, b: factor_b, c: factor_c }
}

#[allow(clippy::cast_lossless)]
pub fn nearest_biome_point_to(x: i64, y: i64) -> (i64, i64) {
    let bp_x = ((x as f32) / (BIOME_SIZE as f32)).floor() as i64 * (BIOME_SIZE as i64);
    let bp_y = ((y as f32) / (BIOME_SIZE as f32)).floor() as i64 * (BIOME_SIZE as i64);

    (bp_x, bp_y)
}
