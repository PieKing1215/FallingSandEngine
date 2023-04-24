use crate::game::common::Rect;

use std::convert::TryInto;

use chunksystem::ChunkKey;
use rapier2d::prelude::{Collider, RigidBody, RigidBodyHandle};
use std::fmt::Debug;

use super::chunk_data::SidedChunkData;
use super::chunk_index::ChunkLocalPosition;
use super::material::color::Color;
use super::mesh::Mesh;
use super::tile_entity::{TileEntity, TileEntityCommon};
use crate::game::common::world::material::MaterialInstance;

pub const CHUNK_SIZE: u16 = 100;
pub const CHUNK_AREA: usize = CHUNK_SIZE as usize * CHUNK_SIZE as usize;
// must be a factor of CHUNK_SIZE
// also (CHUNK_SIZE / LIGHT_SCALE)^2 must be <= 1024 for compute shader (and local_size needs to be set to CHUNK_SIZE / LIGHT_SCALE in the shader)
pub const LIGHT_SCALE: u8 = 4;

pub trait Chunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self
    where
        Self: Sized;

    fn chunk_x(&self) -> i32;
    fn chunk_y(&self) -> i32;

    fn state(&self) -> ChunkState;
    fn set_state(&mut self, state: ChunkState);

    fn dirty_rect(&self) -> Option<Rect<i32>>;
    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>);

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>);
    fn pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>>;
    fn pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>>;
    fn set_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>);
    fn colors_mut(&mut self) -> &mut [Color; CHUNK_AREA];
    fn colors(&self) -> &[Color; CHUNK_AREA];
    fn lights_mut(&mut self) -> &mut [[f32; 4]; CHUNK_AREA];
    fn lights(&self) -> &[[f32; 4]; CHUNK_AREA];
    fn set_background_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>);
    fn background_pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>>;
    fn background_pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>>;
    fn set_background_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>);
    fn background_colors_mut(&mut self) -> &mut [Color; CHUNK_AREA];
    fn background_colors(&self) -> &[Color; CHUNK_AREA];

    fn generate_mesh(&mut self) -> Result<(), String>;
    // fn get_tris(&self) -> &Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>>;
    fn mesh_loops(&self) -> &Option<Mesh>;
    fn rigidbody(&self) -> &Option<ChunkRigidBodyState>;
    fn rigidbody_mut(&mut self) -> &mut Option<ChunkRigidBodyState>;
    fn set_rigidbody(&mut self, body: Option<ChunkRigidBodyState>);

    fn mark_dirty(&mut self);

    fn refresh(&mut self);

    fn set_pixel(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) -> Result<(), String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn set_pixel_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance);

    fn pixel(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn pixel_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance;

    fn replace_pixel<F>(&mut self, pos: ChunkLocalPosition, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>;

    fn set_light(&mut self, pos: ChunkLocalPosition, light: [f32; 3]) -> Result<(), String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn set_light_unchecked(&mut self, pos: ChunkLocalPosition, light: [f32; 3]);

    fn light(&self, pos: ChunkLocalPosition) -> Result<&[f32; 3], String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn light_unchecked(&self, pos: ChunkLocalPosition) -> &[f32; 3];

    fn set_color(&mut self, pos: ChunkLocalPosition, color: Color);
    fn color(&self, pos: ChunkLocalPosition) -> Color;

    fn set_background(
        &mut self,
        pos: ChunkLocalPosition,
        mat: MaterialInstance,
    ) -> Result<(), String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn set_background_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance);

    fn background(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String>;
    /// # Safety
    /// Chunk must be loaded
    unsafe fn background_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance;

    fn add_tile_entity(&mut self, te: TileEntityCommon);

    fn common_tile_entities(&self) -> Box<dyn Iterator<Item = &TileEntityCommon> + '_>;
    fn common_tile_entities_mut(&mut self) -> Box<dyn Iterator<Item = &mut TileEntityCommon> + '_>;

    #[profiling::function]
    fn apply_diff(&mut self, diff: &[(u16, u16, MaterialInstance)]) {
        for (x, y, mat) in diff {
            self.set_pixel((*x, *y).try_into().unwrap(), mat.clone())
                .unwrap(); // TODO: handle this Err
        }
    }
}

pub trait SidedChunk: Chunk {
    type S: SidedChunkData;

    fn sided_tile_entities(&self) -> &[TileEntity<<Self::S as SidedChunkData>::TileEntityData>];
    fn sided_tile_entities_mut(
        &mut self,
    ) -> &mut [TileEntity<<Self::S as SidedChunkData>::TileEntityData>];
    fn sided_tile_entities_removable(
        &mut self,
    ) -> &mut Vec<TileEntity<<Self::S as SidedChunkData>::TileEntityData>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    NotGenerated,
    Generating(u8), // stage
    Cached,
    Active,
}

#[warn(clippy::large_enum_variant)]
pub enum ChunkRigidBodyState {
    Active(RigidBodyHandle),
    Inactive(Box<RigidBody>, Vec<Collider>),
}

#[derive(Default)]
pub struct PassThroughHasherU32(u32);

impl std::hash::Hasher for PassThroughHasherU32 {
    fn finish(&self) -> u64 {
        u64::from(self.0)
    }

    fn write_u32(&mut self, k: u32) {
        self.0 = k;
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("NopHasherU32 only supports u32")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::Rng;

    #[test]
    fn chunk_index_correct() {
        // center
        assert_eq!(chunk_index(0, 0), 0);
        assert_eq!(chunk_index(1, 0), 3);
        assert_eq!(chunk_index(0, 1), 5);
        assert_eq!(chunk_index(1, 1), 12);
        assert_eq!(chunk_index(-1, 0), 1);
        assert_eq!(chunk_index(0, -1), 2);
        assert_eq!(chunk_index(-1, -1), 4);
        assert_eq!(chunk_index(1, -1), 7);
        assert_eq!(chunk_index(-1, 1), 8);

        // some random nearby ones
        assert_eq!(chunk_index(207, 432), 818_145);
        assert_eq!(chunk_index(285, -65), 244_779);
        assert_eq!(chunk_index(958, 345), 3_397_611);
        assert_eq!(chunk_index(632, 255), 1_574_935);
        assert_eq!(chunk_index(-942, 555), 4_481_631);
        assert_eq!(chunk_index(696, 589), 3_304_913);
        assert_eq!(chunk_index(-201, -623), 1_356_726);
        assert_eq!(chunk_index(741, 283), 2_098_742);
        assert_eq!(chunk_index(-302, 718), 2_081_216);
        assert_eq!(chunk_index(493, 116), 742_603);

        // some random far ones
        assert_eq!(chunk_index(1258, 7620), 157_661_886);
        assert_eq!(chunk_index(9438, 4645), 396_685_151);
        assert_eq!(chunk_index(6852, -7129), 390_936_998);
        assert_eq!(chunk_index(-7692, -912), 148_033_644);
        assert_eq!(chunk_index(-4803, -131), 48_674_172);
        assert_eq!(chunk_index(-4565, 8366), 334_425_323);
        assert_eq!(chunk_index(248, -126), 279_629);
        assert_eq!(chunk_index(-1125, 3179), 37_050_886);
        assert_eq!(chunk_index(4315, -4044), 139_745_490);
        assert_eq!(chunk_index(-3126, 9730), 330_560_076);

        // maximum
        assert_eq!(chunk_index(-27804, 18537), u32::MAX);
    }

    #[test]
    fn chunk_index_inv_correct() {
        // center
        assert_eq!(chunk_index_inv(0), (0, 0));
        assert_eq!(chunk_index_inv(3), (1, 0));
        assert_eq!(chunk_index_inv(5), (0, 1));
        assert_eq!(chunk_index_inv(12), (1, 1));
        assert_eq!(chunk_index_inv(1), (-1, 0));
        assert_eq!(chunk_index_inv(2), (0, -1));
        assert_eq!(chunk_index_inv(4), (-1, -1));
        assert_eq!(chunk_index_inv(7), (1, -1));
        assert_eq!(chunk_index_inv(8), (-1, 1));

        // some random nearby ones
        assert_eq!(chunk_index_inv(818_145), (207, 432));
        assert_eq!(chunk_index_inv(244_779), (285, -65));
        assert_eq!(chunk_index_inv(3_397_611), (958, 345));
        assert_eq!(chunk_index_inv(1_574_935), (632, 255));
        assert_eq!(chunk_index_inv(4_481_631), (-942, 555));
        assert_eq!(chunk_index_inv(3_304_913), (696, 589));
        assert_eq!(chunk_index_inv(1_356_726), (-201, -623));
        assert_eq!(chunk_index_inv(2_098_742), (741, 283));
        assert_eq!(chunk_index_inv(2_081_216), (-302, 718));
        assert_eq!(chunk_index_inv(742_603), (493, 116));

        // some random far ones
        assert_eq!(chunk_index_inv(157_661_886), (1258, 7620));
        assert_eq!(chunk_index_inv(396_685_151), (9438, 4645));
        assert_eq!(chunk_index_inv(390_936_998), (6852, -7129));
        assert_eq!(chunk_index_inv(148_033_644), (-7692, -912));
        assert_eq!(chunk_index_inv(48_674_172), (-4803, -131));
        assert_eq!(chunk_index_inv(334_425_323), (-4565, 8366));
        assert_eq!(chunk_index_inv(279_629), (248, -126));
        assert_eq!(chunk_index_inv(37_050_886), (-1125, 3179));
        assert_eq!(chunk_index_inv(139_745_490), (4315, -4044));
        assert_eq!(chunk_index_inv(330_560_076), (-3126, 9730));

        // maximum
        assert_eq!(chunk_index_inv(u32::MAX), (-27804, 18537));
    }

    #[test]
    fn chunk_index_correctly_invertible() {
        for _ in 0..1000 {
            let x: i32 = rand::thread_rng().gen_range(-10000..10000);
            let y: i32 = rand::thread_rng().gen_range(-10000..10000);

            println!("Testing ({x}, {y})...");
            let index = chunk_index(x, y);
            let result = chunk_index_inv(index);

            assert_eq!(result, (x, y));
        }
    }

    #[test]
    fn chunk_update_order() {
        for _ in 0..100 {
            let x: i32 = rand::thread_rng().gen_range(-10000..10000);
            let y: i32 = rand::thread_rng().gen_range(-10000..10000);

            println!("Testing ({x}, {y})...");

            let my_order = super::chunk_update_order(x, y);

            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx != 0 || dy != 0 {
                        // surrounding chunks should not be able to update at the same time
                        assert_ne!(super::chunk_update_order(x + dx, y + dy), my_order);
                    }
                }
            }
        }
    }
}

// #[profiling::function]
#[inline]
pub const fn pixel_to_chunk_pos(x: i64, y: i64) -> (i32, i32) {
    // div_euclid is the same as div_floor in this case (div_floor is currenlty unstable)
    (
        x.div_euclid(CHUNK_SIZE as _) as _,
        y.div_euclid(CHUNK_SIZE as _) as _,
    )
}

#[inline]
pub const fn pixel_to_chunk_pos_with_chunk_size(x: i64, y: i64, chunk_size: u16) -> (i32, i32) {
    // div_euclid is the same as div_floor in this case (div_floor is currenlty unstable)
    (
        x.div_euclid(chunk_size as _) as _,
        y.div_euclid(chunk_size as _) as _,
    )
}

#[inline]
pub const fn pixel_to_pos_in_chunk(world_x: i64, world_y: i64) -> ChunkLocalPosition {
    let (chunk_x, chunk_y) = pixel_to_chunk_pos(world_x, world_y);
    unsafe {
        // need to use unchecked for const
        // Safety: math guarantees x and y are 0..CHUNK_SIZE
        ChunkLocalPosition::new_unchecked(
            (world_x - chunk_x as i64 * CHUNK_SIZE as i64) as u16,
            (world_y - chunk_y as i64 * CHUNK_SIZE as i64) as u16,
        )
    }
}

#[inline]
pub const fn pixel_to_chunk(world_x: i64, world_y: i64) -> (ChunkKey, ChunkLocalPosition) {
    let (chunk_x, chunk_y) = pixel_to_chunk_pos(world_x, world_y);
    ((chunk_x, chunk_y), unsafe {
        // need to use unchecked for const
        // Safety: math guarantees x and y are 0..CHUNK_SIZE
        ChunkLocalPosition::new_unchecked(
            (world_x - chunk_x as i64 * CHUNK_SIZE as i64) as u16,
            (world_y - chunk_y as i64 * CHUNK_SIZE as i64) as u16,
        )
    })
}

#[inline]
pub fn chunk_index(chunk_x: i32, chunk_y: i32) -> u32 {
    #[inline]
    const fn int_to_nat(i: i32) -> u32 {
        if i >= 0 {
            (2 * i) as u32
        } else {
            (-2 * i - 1) as u32
        }
    }
    let xx: u32 = int_to_nat(chunk_x);
    let yy: u32 = int_to_nat(chunk_y);

    // TODO: this multiply is the first thing to overflow if you go out too far
    //          (though you need to go out ~32768 chunks (2^16 / 2)
    ((u64::from(xx + yy) * u64::from(xx + yy + 1)) / 2 + u64::from(yy)) as u32
}

#[inline]
pub fn chunk_index_inv(index: u32) -> (i32, i32) {
    let w = (((8 * u64::from(index) + 1) as f64).sqrt() - 1.0).floor() as u64 / 2;
    let t = (w * w + w) / 2;
    let yy = u64::from(index) - t;
    let xx = w - yy;
    const fn nat_to_int(i: u64) -> i32 {
        if i % 2 == 0 {
            (i / 2) as i32
        } else {
            -((i / 2 + 1) as i32)
        }
    }
    let x = nat_to_int(xx);
    let y = nat_to_int(yy);

    (x, y)
}

#[inline]
pub const fn chunk_update_order(chunk_x: i32, chunk_y: i32) -> u8 {
    let yy = (-chunk_y).rem_euclid(2) as u8;
    let xx = chunk_x.rem_euclid(2) as u8;

    yy * 2 + xx
}
