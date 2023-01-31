use std::{fmt::Debug, sync::Arc};

use rand::Rng;
use simdnoise::NoiseBuilder;

use crate::game::{
    common::world::{
        gen::{
            feature::{
                placement_mods::material_match::MaterialMatchFn, ConfiguredFeature, ProviderFn,
            },
            populator::ChunkContext,
        },
        material::placer::MaterialPlacerID,
        CHUNK_SIZE,
    },
    Registries,
};

pub struct Blob {
    placer_id: MaterialPlacerID,
    radius: Arc<ProviderFn<u8>>,
    replace: Arc<MaterialMatchFn>,
}

impl Blob {
    pub fn new(
        placer_id: MaterialPlacerID,
        radius: Arc<ProviderFn<u8>>,
        replace: Arc<MaterialMatchFn>,
    ) -> Self {
        Self { placer_id, radius, replace }
    }
}

impl Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Blob")
            .field("placer_id", &self.placer_id)
            .finish()
    }
}

impl ConfiguredFeature for Blob {
    fn try_place(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        seed: i32,
        rng: &mut dyn rand::RngCore,
        registries: &Registries,
    ) {
        let (chunk_x, chunk_y) = chunks.center_chunk();
        let chunk_pixel_x = chunk_x * i32::from(CHUNK_SIZE);
        let chunk_pixel_y = chunk_y * i32::from(CHUNK_SIZE);
        let cofs_x = chunk_pixel_x as f32 + pos.0 as f32;
        let cofs_y = chunk_pixel_y as f32 + pos.1 as f32;

        let placer = &registries.material_placers.get(&self.placer_id).unwrap().1;

        let radius = (self.radius)(rng);
        let alt_radius = (f32::from(radius) * rng.gen_range(0.5..1.0)) as u8;

        let (radius_x, radius_y) = if rng.gen_bool(0.5) {
            (radius, alt_radius)
        } else {
            (alt_radius, radius)
        };

        let noise = NoiseBuilder::gradient_2d_offset(
            cofs_x - f32::from(radius_x),
            (radius_x as usize) * 2 + 1,
            cofs_y - f32::from(radius_y),
            (radius_y as usize) * 2 + 1,
        )
        .with_freq(0.05)
        .with_seed(seed)
        .generate();

        // log::debug!("{} {}", noise.1, noise.2);

        for dx in -i32::from(radius_x)..=i32::from(radius_x) {
            for dy in -i32::from(radius_y)..=i32::from(radius_y) {
                let n = noise.0[((dx + i32::from(radius_x))
                    + (dy + i32::from(radius_y)) * (i32::from(radius_x) * 2 + 1))
                    as usize];
                let ef_dx = dx as f32 * (f32::from(radius) / f32::from(radius_x));
                let ef_dy = dy as f32 * (f32::from(radius) / f32::from(radius_y));
                let dst = (ef_dx * ef_dx + ef_dy * ef_dy).sqrt() * 1.25
                    + n * f32::from(radius) / 0.025 * 0.33;
                let f = 1.0 - (dst / f32::from(radius));

                if f > 0.0 {
                    let x = pos.0 + dx;
                    let y = pos.1 + dy;

                    if (self.replace)(chunks.get(x, y).unwrap()) {
                        let mat = placer.pixel(
                            i64::from(chunk_pixel_x) + i64::from(x),
                            i64::from(chunk_pixel_y) + i64::from(y),
                        );

                        chunks.set(x, y, mat).unwrap();
                    }
                }
            }
        }
    }
}
