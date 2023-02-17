use bracket_noise::prelude::{FastNoise, NoiseType};
use simdnoise::NoiseBuilder;

use crate::game::common::{
    world::{material::MaterialInstance, CHUNK_SIZE},
    Registries,
};

use super::{ChunkContext, Populator};

pub struct CavePopulator;

impl Populator<0> for CavePopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<0>, seed: i32, _registries: &Registries) {
        let (chunk_x, chunk_y) = chunks.center_chunk();
        let chunk_pixel_x = chunk_x * i32::from(CHUNK_SIZE);
        let chunk_pixel_y = chunk_y * i32::from(CHUNK_SIZE);
        let cofs_x = chunk_pixel_x as f32;
        let cofs_y = chunk_pixel_y as f32;

        let enable_caves = true;

        if enable_caves {
            // based on techniques discussed here: https://accidentalnoise.sourceforge.net/minecraftworlds.html

            let turbulance_scale = 100 * 15;

            // offsetting by seed is a workaround for https://github.com/verpeteren/rust-simd-noise/issues/42
            let noise_turbulance = NoiseBuilder::fbm_2d_offset(
                cofs_x + seed as f32 / 100_000.0,
                CHUNK_SIZE.into(),
                cofs_y,
                CHUNK_SIZE.into(),
            )
            .with_octaves(6)
            .with_lacunarity(2.0)
            .with_gain(0.5)
            .with_freq(0.00075)
            .with_seed(seed)
            .generate();

            let mut noise_ridge = FastNoise::seeded(seed as _);
            noise_ridge.set_noise_type(NoiseType::Simplex);
            noise_ridge.set_fractal_octaves(1);
            noise_ridge.set_frequency(0.0005);

            {
                profiling::scope!("loop");
                for x in 0..i32::from(CHUNK_SIZE) {
                    for y in 0..i32::from(CHUNK_SIZE) {
                        let i = (x + y * i32::from(CHUNK_SIZE)) as usize;
                        let tu = (noise_turbulance.0[i] - -0.03) / 0.06;

                        let t_ofs = (tu * turbulance_scale as f32)
                            .clamp(-(turbulance_scale as f32), turbulance_scale as f32);
                        let t_x = x as f32 + t_ofs;
                        let t_y = y as f32;

                        let nv = noise_ridge.get_noise(cofs_x + t_x, cofs_y + t_y);

                        let f = 1.0 - (nv.abs()).clamp(0.0, 1.0);
                        if f > 0.8 {
                            chunks.set(x, y, MaterialInstance::air()).unwrap();
                        }

                        // let f = if f > 0.7 { 0.0 } else { 1.0 };
                        // let mut m = *chunks.get(x, y).unwrap();
                        // m.color = crate::game::common::world::material::color::Color::rgba(f, f, f, 1.0);
                        // chunks.set(x, y, m).unwrap();
                    }
                }
            }
        }
    }
}
