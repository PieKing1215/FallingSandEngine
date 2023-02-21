pub mod placement;

use simdnoise::NoiseBuilder;

use crate::game::common::{
    registry::{Registry, RegistryID},
    world::{
        material::{
            self,
            color::Color,
            placer::{self, MaterialPlacer, MaterialPlacerSampler},
            MaterialInstance, PhysicsType,
        },
        pixel_to_chunk_pos, CHUNK_SIZE,
    },
    FileHelper, Registries,
};

use self::placement::{
    biome_params_at, nearest_biome_point_to, single_random_at, BiomePlacementParameter, BIOME_SIZE,
};

pub enum MaterialPlacerIDOrMaterialInstance {
    MaterialPlacer(RegistryID<MaterialPlacer>),
    MaterialInstance(MaterialInstance),
}

impl MaterialPlacerIDOrMaterialInstance {
    pub fn as_placer<'a>(&'a self, registries: &'a Registries) -> &dyn MaterialPlacerSampler {
        match self {
            MaterialPlacerIDOrMaterialInstance::MaterialPlacer(pl) => {
                registries.material_placers.get(pl).unwrap()
            },
            MaterialPlacerIDOrMaterialInstance::MaterialInstance(mat) => mat,
        }
    }
}

impl From<RegistryID<MaterialPlacer>> for MaterialPlacerIDOrMaterialInstance {
    fn from(value: RegistryID<MaterialPlacer>) -> Self {
        Self::MaterialPlacer(value)
    }
}

impl From<MaterialInstance> for MaterialPlacerIDOrMaterialInstance {
    fn from(value: MaterialInstance) -> Self {
        Self::MaterialInstance(value)
    }
}

pub struct Biome {
    pub placement: BiomePlacementParameter,
    pub base_placer: MaterialPlacerIDOrMaterialInstance,
}

pub type BiomeRegistry = Registry<Biome>;

impl BiomeRegistry {
    pub fn nearest(&self, test: BiomePlacementParameter) -> (&RegistryID<Biome>, &Biome) {
        self.into_iter()
            .min_by(|(_, a), (_, b)| {
                test.dist_sq(&a.placement)
                    .partial_cmp(&test.dist_sq(&b.placement))
                    .unwrap()
            })
            .unwrap()
    }

    pub fn biome_at(&self, x: i64, y: i64, seed: i32) -> (&RegistryID<Biome>, &Biome) {
        self.biome_block::<1, 1>(x, y, seed)[0]
    }

    /// The returned `Vec` will always have size `W * H`.
    ///
    /// TODO: Once `generic_const_exprs` is stabilized this could change to an array,
    pub fn biome_block<const W: u16, const H: u16>(
        &self,
        x: i64,
        y: i64,
        seed: i32,
    ) -> Vec<(&RegistryID<Biome>, &Biome)> {
        let (chunk_x, chunk_y) = pixel_to_chunk_pos(x, y);

        let (center_biome_point_x, center_biome_point_y) = nearest_biome_point_to(
            (i64::from(chunk_x) * i64::from(CHUNK_SIZE)) + i64::from(CHUNK_SIZE / 2),
            (i64::from(chunk_y) * i64::from(CHUNK_SIZE)) + i64::from(CHUNK_SIZE / 2),
        );

        let base_pts = (-2..=2)
            .flat_map(|x| (-2..=2).map(move |y| (x, y)))
            .map(|(x, y)| {
                (
                    center_biome_point_x + x * i64::from(BIOME_SIZE),
                    center_biome_point_y + y * i64::from(BIOME_SIZE),
                )
            })
            .collect::<Vec<_>>();

        let vals = base_pts
            .iter()
            .map(|(x, y)| {
                let disp_x = x
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 1)
                        * 20.0
                        * f32::from(BIOME_SIZE)) as i64;
                let disp_y = y
                    + (single_random_at(*x as f32, *y as f32, 0.003, seed + 2)
                        * 20.0
                        * f32::from(BIOME_SIZE)) as i64;

                (
                    (disp_x, disp_y),
                    self.nearest(biome_params_at(*x, *y, seed)),
                )
            })
            .collect::<Vec<_>>();

        let ofs_x_1 = NoiseBuilder::gradient_2d_offset(x as f32, W.into(), y as f32, H.into())
            .with_freq(0.005)
            .with_seed(seed + 3)
            .generate()
            .0;

        let ofs_y_1 = NoiseBuilder::gradient_2d_offset(x as f32, W.into(), y as f32, H.into())
            .with_freq(0.005)
            .with_seed(seed + 4)
            .generate()
            .0;

        let ofs_x_2 = NoiseBuilder::gradient_2d_offset(x as f32, W.into(), y as f32, H.into())
            .with_freq(0.015)
            .with_seed(seed + 3)
            .generate()
            .0;

        let ofs_y_2 = NoiseBuilder::gradient_2d_offset(x as f32, W.into(), y as f32, H.into())
            .with_freq(0.015)
            .with_seed(seed + 4)
            .generate()
            .0;

        (0..(W * H) as usize)
            .map(|i| {
                let rel_x = i % (W as usize);
                let rel_y = i / (W as usize);

                let biome = vals
                    .iter()
                    .min_by(|((x1, y1), _v1), ((x2, y2), _v2)| unsafe {
                        let ox1 = ofs_x_1.get_unchecked(i) * 1000.0;
                        let ox2 = ofs_x_2.get_unchecked(i) * 500.0;
                        let oy1 = ofs_y_1.get_unchecked(i) * 1000.0;
                        let oy2 = ofs_y_2.get_unchecked(i) * 500.0;

                        let ox = rel_x as i64 + x + (ox1 + ox2) as i64;
                        let oy = rel_y as i64 + y + (oy1 + oy2) as i64;

                        let dx1 = x1 - ox;
                        let dy1 = y1 - oy;
                        let d1 = dx1 * dx1 + dy1 * dy1;

                        let dx2 = x2 - ox;
                        let dy2 = y2 - oy;
                        let d2 = dx2 * dx2 + dy2 * dy2;

                        d1.cmp(&d2)
                    })
                    .unwrap()
                    .1;

                biome
            })
            .collect()
    }
}

pub fn init_biomes(_file_helper: &FileHelper) -> BiomeRegistry {
    let mut registry = BiomeRegistry::new();

    registry.register(
        "main",
        Biome {
            placement: [0.5, 0.5, 0.5].into(),
            base_placer: placer::SMOOTH_STONE.clone().into(),
        },
    );

    registry.register(
        "dirt",
        Biome {
            placement: [0.0, 0.0, 0.0].into(),
            base_placer: placer::SMOOTH_DIRT.clone().into(),
        },
    );

    registry.register(
        "red",
        Biome {
            placement: [0.75, 0.0, 0.0].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_red").into(),
        },
    );
    registry.register(
        "green",
        Biome {
            placement: [0.0, 0.75, 0.0].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_green").into(),
        },
    );
    registry.register(
        "blue",
        Biome {
            placement: [0.0, 0.0, 0.75].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_blue").into(),
        },
    );
    registry.register(
        "cyan",
        Biome {
            placement: [0.25, 1.0, 1.0].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_cyan").into(),
        },
    );
    registry.register(
        "magenta",
        Biome {
            placement: [1.0, 0.25, 1.0].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_magenta").into(),
        },
    );
    registry.register(
        "yellow",
        Biome {
            placement: [1.0, 1.0, 0.25].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_yellow").into(),
        },
    );
    registry.register(
        "white",
        Biome {
            placement: [1.0, 1.0, 1.0].into(),
            base_placer: RegistryID::<MaterialPlacer>::from("test_white").into(),
        },
    );

    registry
}
