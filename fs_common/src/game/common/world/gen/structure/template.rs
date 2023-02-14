use crate::game::common::{
    world::{copy_paste::MaterialBuf, gen::structure::AngleMod, Chunk, ChunkHandler},
    Rect,
};

use super::{pool::StructurePoolID, Direction};

#[derive(Debug, Clone)]
pub struct StructureTemplate {
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
    pub pool: StructurePoolID,
    /// If `true`, this node will still try to generate a child even if depth is at 0
    pub depth_override: bool,
    pub block_in_dirs: Option<Vec<Direction>>,
}

impl StructureNodeConfig {
    pub fn new(pool: StructurePoolID) -> Self {
        Self { pool, depth_override: false, block_in_dirs: None }
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
}

impl StructureTemplate {
    #[allow(clippy::type_complexity)]
    pub fn options(
        &self,
        origin: (i64, i64),
        dir_in: Direction,
    ) -> Vec<(
        Rect<i64>,
        Vec<(StructureNodeGlobalPlacement, StructureNodeConfig)>,
    )> {
        #[inline]
        #[must_use]
        fn rotated(rect: Rect<i64>, pivot: (i64, i64), angle: AngleMod) -> Rect<i64> {
            let (x1_r, y1_r) = rotate_point((rect.x1, rect.y1), pivot, angle.degrees());
            let (x2_r, y2_r) = rotate_point((rect.x2, rect.y2), pivot, angle.degrees());

            Rect::new(
                x1_r.min(x2_r),
                y1_r.min(y2_r),
                x1_r.max(x2_r),
                y1_r.max(y2_r),
            )
        }

        #[inline]
        fn rotate_point(point: (i64, i64), pivot: (i64, i64), deg: f32) -> (i64, i64) {
            let sin = deg.to_radians().sin();
            let cos = deg.to_radians().cos();
            (
                (cos * (point.0 - pivot.0) as f32 - sin * (point.1 - pivot.1) as f32
                    + pivot.0 as f32) as i64,
                (sin * (point.0 - pivot.0) as f32
                    + cos * (point.1 - pivot.1) as f32
                    + pivot.1 as f32) as i64,
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
                    let (dst_x, dst_y) = rotate_point((src_x, src_y), origin, angle.degrees());
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

            opts.push((bounds, children));
        }

        opts
    }

    pub fn place<C: Chunk + Send>(
        &self,
        _chunk_handler: &mut ChunkHandler<C>,
        _origin: (i64, i64),
        _dir_in: Direction,
    ) {
        // TODO
        todo!();
        // self.buf.paste(chunk_handler, origin.0, origin.1).unwrap();
    }
}
