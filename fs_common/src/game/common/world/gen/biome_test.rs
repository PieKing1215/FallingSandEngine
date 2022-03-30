use crate::game::common::world::material::MaterialInstance;

use simdnoise::NoiseBuilder;

use crate::game::common::world::CHUNK_SIZE;

use super::{biome::BiomePlacementParameter, WorldGenerator};

#[derive(Clone, Copy, Debug)]
pub struct BiomeTestGenerator;

const BIOME_SIZE: u16 = 200;

fn single_random_at(x: f32, y: f32, freq: f32, seed: i32) -> f32 {
    NoiseBuilder::gradient_2d_offset(x, 1, y, 1)
        .with_freq(freq)
        .with_seed(seed)
        .generate()
        .0[0]
}

fn biome_params_at(x: i32, y: i32, seed: i32) -> BiomePlacementParameter {
    let factor_a =
        (single_random_at(x as f32, y as f32, 0.001, seed + 4) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_b =
        (single_random_at(x as f32, y as f32, 0.0005, seed + 5) * 20.0 + 0.5).clamp(0.0, 1.0);
    let factor_c =
        (single_random_at(x as f32, y as f32, 0.00025, seed + 6) * 20.0 + 0.5).clamp(0.0, 1.0);
    BiomePlacementParameter { a: factor_a, b: factor_b, c: factor_c }
}

#[allow(clippy::cast_lossless)]
fn nearest_biome_point_to(x: i32, y: i32) -> (i32, i32) {
    let bp_x = ((x as f32) / (BIOME_SIZE as f32))
        .floor() as i32
        * (BIOME_SIZE as i32);
    let bp_y = ((y as f32) / (BIOME_SIZE as f32))
        .floor() as i32
        * (BIOME_SIZE as i32);

    (bp_x, bp_y)
}

impl WorldGenerator for BiomeTestGenerator {
    #[allow(clippy::cast_lossless)]
    #[profiling::function]
    fn generate(
        &self,
        chunk_x: i32,
        chunk_y: i32,
        seed: i32,
        pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize],
    ) {
        let chunk_pixel_x = chunk_x * CHUNK_SIZE as i32;
        let chunk_pixel_y = chunk_y * CHUNK_SIZE as i32;
        let cofs_x = chunk_pixel_x as f32;
        let cofs_y = chunk_pixel_y as f32;

        let (center_biome_point_x, center_biome_point_y) = nearest_biome_point_to((chunk_x * CHUNK_SIZE as i32) + (CHUNK_SIZE / 2) as i32, (chunk_y * CHUNK_SIZE as i32) + (CHUNK_SIZE / 2) as i32);

        let base_pts = (-2..=2)
            .flat_map(|x| (-2..=2).map(move |y| (x, y)))
            .map(|(x, y)| {
                (
                    center_biome_point_x + x * BIOME_SIZE as i32,
                    center_biome_point_y + y * BIOME_SIZE as i32,
                )
            })
            .collect::<Vec<_>>();

        let mut vals = base_pts
            .iter()
            .map(|(x, y)| {
                let disp_x = x
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 1)
                        * 20.0
                        * BIOME_SIZE as f32) as i32;
                let disp_y = y
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 2)
                        * 20.0
                        * BIOME_SIZE as f32) as i32;

                let biome = (*super::biome::test::TEST_BIOME_PLACEMENT)
                    .nearest(biome_params_at(*x, *y, seed));

                ((disp_x, disp_y), biome)
            })
            .collect::<Vec<_>>();

        let ofs_x_1 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.005)
                .with_seed(seed + 3)
                .generate()
                .0;

        let ofs_y_1 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.005)
                .with_seed(seed + 4)
                .generate()
                .0;

        let ofs_x_2 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.015)
                .with_seed(seed + 3)
                .generate()
                .0;

        let ofs_y_2 =
            NoiseBuilder::gradient_2d_offset(cofs_x, CHUNK_SIZE.into(), cofs_y, CHUNK_SIZE.into())
                .with_freq(0.015)
                .with_seed(seed + 4)
                .generate()
                .0;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                let i = (x + y * CHUNK_SIZE) as usize;

                vals.sort_by(|((x1, y1), _v1), ((x2, y2), _v2)| {
                    let dx1 = x1
                        - (x as i32
                            + cofs_x as i32
                            + (ofs_x_1[i] * 1000.0 + ofs_x_2[i] * 500.0) as i32);
                    let dy1 = y1
                        - (y as i32
                            + cofs_y as i32
                            + (ofs_y_1[i] * 1000.0 + ofs_y_2[i] * 500.0) as i32);
                    let d1 = dx1 * dx1 + dy1 * dy1;

                    let dx2 = x2
                        - (x as i32
                            + cofs_x as i32
                            + (ofs_x_1[i] * 1000.0 + ofs_x_2[i] * 500.0) as i32);
                    let dy2 = y2
                        - (y as i32
                            + cofs_y as i32
                            + (ofs_y_1[i] * 1000.0 + ofs_y_2[i] * 500.0) as i32);
                    let d2 = dx2 * dx2 + dy2 * dy2;

                    d1.cmp(&d2)
                });

                let biome = vals.first().unwrap().1;

                pixels[i] = biome.pixel();
                colors[i * 4] = pixels[i].color.r;
                colors[i * 4 + 1] = pixels[i].color.g;
                colors[i * 4 + 2] = pixels[i].color.b;
                colors[i * 4 + 3] = pixels[i].color.a;
            }
        }
    }

    fn max_gen_stage(&self) -> u8 {
        2
    }
}
