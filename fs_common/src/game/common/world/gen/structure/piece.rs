use std::fs;

use asefile::AsepriteFile;
use image::{DynamicImage, GenericImageView};

use crate::game::common::{
    registry::{Registry, RegistryID},
    world::{
        copy_paste::MaterialBuf,
        gen::structure::AngleMod,
        material::{self, color::Color, Material, MaterialInstance, PhysicsType},
        ChunkHandlerGeneric,
    },
    FileHelper, Rect,
};

use super::{pool::StructurePool, Direction};

#[derive(Debug, Clone)]
pub struct StructurePiece {
    pub buf: MaterialBuf,
    pub child_nodes: Vec<(StructureNodeLocalPlacement, StructureNodeConfig)>,
}

#[derive(Debug, Clone)]
pub struct StructureNodeLocalPlacement {
    pub x: u32,
    pub y: u32,
    pub direction_out: Direction,
}

#[derive(Debug, Clone)]
pub struct StructureNodeGlobalPlacement {
    pub x: i64,
    pub y: i64,
    pub direction_out: Direction,
}

#[derive(Debug, Clone)]
pub struct StructureNodeConfig {
    pub pool: RegistryID<StructurePool>,
    pub fallback_pool: Option<RegistryID<StructurePool>>,
    /// If `true`, this node will still try to generate a child even if depth is at 0
    pub depth_override: bool,
    pub block_in_dirs: Option<Vec<Direction>>,
}

impl StructureNodeConfig {
    pub fn new(pool: impl Into<RegistryID<StructurePool>>) -> Self {
        Self {
            pool: pool.into(),
            fallback_pool: None,
            depth_override: false,
            block_in_dirs: None,
        }
    }

    #[must_use]
    pub fn override_depth(mut self) -> Self {
        self.depth_override = true;
        self
    }

    #[must_use]
    pub fn block_in_dirs(mut self, dirs: Vec<Direction>) -> Self {
        self.block_in_dirs = Some(dirs);
        self
    }

    #[must_use]
    pub fn fallback_pool(mut self, pool: impl Into<RegistryID<StructurePool>>) -> Self {
        self.fallback_pool = Some(pool.into());
        self
    }
}

type PlaceFn = dyn Fn(&StructurePiece, &mut dyn ChunkHandlerGeneric) -> Result<(), String>;

impl StructurePiece {
    #[allow(clippy::type_complexity)]
    pub fn options(
        &self,
        origin: (i64, i64),
        dir_in: Direction,
    ) -> Vec<(
        Rect<i64>,
        Vec<(StructureNodeGlobalPlacement, StructureNodeConfig)>,
        Box<PlaceFn>,
    )> {
        #[inline]
        #[must_use]
        fn rotated(rect: Rect<i64>, pivot: (i64, i64), angle: AngleMod) -> Rect<i64> {
            let (x1_r, y1_r) = angle.rotate_point((rect.x1, rect.y1), pivot);
            let (x2_r, y2_r) = angle.rotate_point((rect.x2, rect.y2), pivot);

            Rect::new(
                x1_r.min(x2_r),
                y1_r.min(y2_r),
                x1_r.max(x2_r),
                y1_r.max(y2_r),
            )
        }

        let mut opts = vec![];

        for i in 0..self.child_nodes.len() {
            let (placement, config) = &self.child_nodes[i];

            if config
                .block_in_dirs
                .as_ref()
                .map_or(false, |block| block.contains(&dir_in))
            {
                continue;
            }

            let ofs_x = i64::from(placement.x);
            let ofs_y = i64::from(placement.y);
            let src = Rect::new_wh(
                origin.0 - ofs_x,
                origin.1 - ofs_y,
                i64::from(self.buf.width),
                i64::from(self.buf.height),
            );

            let angle = placement.direction_out.angle(dir_in.opposite());

            // log::debug!("{:?} {:?} {:?}", placement.direction_out, dir_in.opposite(), angle);

            let bounds = rotated(src, origin, angle);

            let children = self
                .child_nodes
                .iter()
                .enumerate()
                .filter(|(ci, _)| *ci != i)
                .map(|(_, (ch_placement, config))| {
                    let src_x = src.x1 + i64::from(ch_placement.x);
                    let src_y = src.y1 + i64::from(ch_placement.y);
                    let (dst_x, dst_y) = angle.rotate_point((src_x, src_y), origin);
                    (
                        StructureNodeGlobalPlacement {
                            x: dst_x,
                            y: dst_y,
                            direction_out: ch_placement.direction_out.rotated(angle),
                        },
                        config.clone(),
                    )
                })
                .collect();

            opts.push((
                bounds,
                children,
                Box::new(
                    move |st: &Self, chunk_handler: &mut dyn ChunkHandlerGeneric| {
                        st.buf
                            .rotated(angle)
                            .paste(chunk_handler, bounds.left(), bounds.top())
                    },
                ) as Box<PlaceFn>,
            ));
        }

        opts
    }
}

// registry

pub type StructurePieceRegistry = Registry<StructurePiece>;

#[allow(clippy::too_many_lines)]
pub fn init_structure_pieces(file_helper: &FileHelper) -> StructurePieceRegistry {
    let mut registry = Registry::new();

    registry.register(
        "a",
        make_test_structure(
            200,
            150,
            vec![
                (
                    StructureNodeLocalPlacement { x: 0, y: 75, direction_out: Direction::Left },
                    StructureNodeConfig::new("hallways").fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 200, y: 40, direction_out: Direction::Right },
                    StructureNodeConfig::new("hallways").fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 200, y: 110, direction_out: Direction::Right },
                    StructureNodeConfig::new("hallways").fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 40, y: 0, direction_out: Direction::Up },
                    StructureNodeConfig::new("hallways").fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 110, y: 150, direction_out: Direction::Down },
                    StructureNodeConfig::new("hallways").fallback_pool("end_pieces"),
                ),
            ],
        ),
    );
    registry.register(
        "a2",
        make_test_structure(
            200,
            100,
            vec![
                (
                    StructureNodeLocalPlacement { x: 0, y: 50, direction_out: Direction::Left },
                    StructureNodeConfig::new("hallways")
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 200, y: 20, direction_out: Direction::Right },
                    StructureNodeConfig::new("hallways")
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
            ],
        ),
    );

    registry.register(
        "b",
        make_test_structure(
            100,
            32,
            vec![
                (
                    StructureNodeLocalPlacement { x: 0, y: 16, direction_out: Direction::Left },
                    StructureNodeConfig::new("rooms")
                        .override_depth()
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 100, y: 16, direction_out: Direction::Right },
                    StructureNodeConfig::new("rooms")
                        .override_depth()
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
            ],
        ),
    );

    let ase =
        AsepriteFile::read_file(&file_helper.asset_path("data/structure/piece/corner/corner.ase"))
            .unwrap();
    registry.register(
        "b2",
        load_from_ase(
            &ase,
            vec![
                (
                    StructureNodeLocalPlacement { x: 0, y: 60, direction_out: Direction::Left },
                    StructureNodeConfig::new("rooms_or_straight_hallways")
                        .override_depth()
                        .fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 60, y: 0, direction_out: Direction::Up },
                    StructureNodeConfig::new("rooms_or_straight_hallways")
                        .override_depth()
                        .fallback_pool("end_pieces"),
                ),
            ],
        ),
    );

    let ase =
        AsepriteFile::read_file(&file_helper.asset_path("data/structure/piece/stairs/stairs.ase"))
            .unwrap();
    registry.register(
        "stairs",
        load_from_ase(
            &ase,
            vec![
                (
                    StructureNodeLocalPlacement { x: 0, y: 60, direction_out: Direction::Left },
                    StructureNodeConfig::new("rooms_or_straight_hallways")
                        .override_depth()
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
                (
                    StructureNodeLocalPlacement { x: 80, y: 19, direction_out: Direction::Right },
                    StructureNodeConfig::new("rooms_or_straight_hallways")
                        .override_depth()
                        .block_in_dirs(vec![Direction::Up, Direction::Down])
                        .fallback_pool("end_pieces"),
                ),
            ],
        ),
    );

    let data = &fs::read(file_helper.asset_path("data/structure/piece/end_carve.png")).unwrap();
    let img = image::load_from_memory(data).unwrap();
    registry.register(
        "end_carve",
        make_test_structure_from_img(
            &img,
            vec![(
                StructureNodeLocalPlacement { x: 0, y: 11, direction_out: Direction::Left },
                StructureNodeConfig::new("empty"),
            )],
        ),
    );

    let ase = AsepriteFile::read_file(
        &file_helper.asset_path("data/structure/piece/yellow_thing/yellow_thing.ase"),
    )
    .unwrap();
    registry.register(
        "yellow_thing",
        load_from_ase(
            &ase,
            vec![(
                StructureNodeLocalPlacement { x: 25, y: 25, direction_out: Direction::Down },
                StructureNodeConfig::new("empty"),
            )],
        ),
    );

    registry
}

fn make_test_structure(
    w: u16,
    h: u16,
    child_nodes: Vec<(StructureNodeLocalPlacement, StructureNodeConfig)>,
) -> StructurePiece {
    let mut buf = MaterialBuf::new(w, h, vec![MaterialInstance::air(); (w * h) as usize]).unwrap();

    for x in 0..w {
        for y in 0..h {
            let near_node = child_nodes.iter().any(|cn| {
                let dx = cn.0.x.abs_diff(u32::from(x));
                let dy = cn.0.y.abs_diff(u32::from(y));
                dx < 12 && dy < 12
            });
            if (x < 4 || y < 4 || (x >= w - 4) || (y >= h - 4)) && !near_node {
                buf.set(
                    x,
                    y,
                    material::TEST.instance(
                        PhysicsType::Solid,
                        Color::rgb(
                            f32::from(x) / f32::from(w),
                            f32::from(y) / f32::from(h),
                            0.0,
                        ),
                    ),
                );
            }
        }
    }

    StructurePiece { buf, child_nodes }
}

fn make_test_structure_from_img(
    img: &DynamicImage,
    child_nodes: Vec<(StructureNodeLocalPlacement, StructureNodeConfig)>,
) -> StructurePiece {
    let w = img.width() as u16;
    let h = img.height() as u16;
    let mut buf = MaterialBuf::new(w, h, vec![MaterialInstance::air(); (w * h) as usize]).unwrap();

    for x in 0..w {
        for y in 0..h {
            let c = img.get_pixel(u32::from(x), u32::from(y));
            if c.0 == [0, 0, 0, 255] {
                buf.set(
                    x,
                    y,
                    material::STRUCTURE_VOID.instance(PhysicsType::Air, Color::rgb(0, 0, 0)),
                );
            } else if c.0[3] > 0 {
                buf.set(
                    x,
                    y,
                    material::TEST.instance(PhysicsType::Solid, Color::rgb(c.0[0], c.0[1], c.0[2])),
                );
            }
        }
    }

    StructurePiece { buf, child_nodes }
}

fn load_from_ase(
    ase: &AsepriteFile,
    child_nodes: Vec<(StructureNodeLocalPlacement, StructureNodeConfig)>,
) -> StructurePiece {
    let w = ase.width() as u16;
    let h = ase.height() as u16;
    let mut buf = MaterialBuf::new(w, h, vec![MaterialInstance::air(); (w * h) as usize]).unwrap();

    for layer in ase.layers() {
        let img = layer.frame(0).image();

        let material_id: RegistryID<Material> = layer.name().into();
        let mut override_color = None;
        let mut phys_type = PhysicsType::Solid;
        let mut light = [0.0, 0.0, 0.0];
        if let Some(user) = layer.user_data() {
            let flags = user
                .text
                .as_ref()
                .map(|t| t.split_whitespace().collect::<Vec<_>>())
                .unwrap_or_default();
            if flags.contains(&"override_color") {
                override_color = user.color;
            }

            if flags.contains(&"lit") {
                let c = user.color.unwrap();
                light = [
                    f32::from(c.0[0]) / f32::from(u8::MAX),
                    f32::from(c.0[1]) / f32::from(u8::MAX),
                    f32::from(c.0[2]) / f32::from(u8::MAX),
                ];
            }

            if flags.contains(&"air") {
                phys_type = PhysicsType::Air;
            } else if flags.contains(&"solid") {
                phys_type = PhysicsType::Solid;
            } else if flags.contains(&"sand") {
                phys_type = PhysicsType::Sand;
            } else if flags.contains(&"liquid") {
                phys_type = PhysicsType::Liquid;
            } else if flags.contains(&"gas") {
                phys_type = PhysicsType::Gas;
            }
        }

        for x in 0..w {
            for y in 0..h {
                let img_color = img.get_pixel(u32::from(x), u32::from(y));
                if img_color.0[3] > 0 {
                    let c = override_color.as_ref().unwrap_or(img_color);
                    buf.set(
                        x,
                        y,
                        material_id
                            .instance(phys_type, Color::rgba(c.0[0], c.0[1], c.0[2], c.0[3]))
                            .with_light(light),
                    );
                }
            }
        }
    }

    StructurePiece { buf, child_nodes }
}
