
pub mod test;
pub mod cave;
pub mod nearby_replace;

use crate::game::common::world::{Chunk, material::MaterialInstance, CHUNK_SIZE};

// where S=0 means 1x1, S=1 means 3x3, etc
pub trait Populator<const S: usize> {
    fn populate(&self, chunks: ChunkContext<S>, seed: i32);
}

// where S=0 means 1x1, S=1 means 3x3, etc
// when generic_const_exprs gets stablized eventually, could use [&mut dyn Chunk; (S * 2 + 1) * (S * 2 + 1)]
pub struct ChunkContext<'a, const S: usize> (&'a mut [&'a mut dyn Chunk]);

impl<'a, const S: usize> ChunkContext<'a, S> {
    #[warn(clippy::result_unit_err)] // TODO
    pub fn new(slice: &'a mut [&'a mut dyn Chunk]) -> Result<Self, ()> {
        if slice.len() == (S * 2 + 1) * (S * 2 + 1) {
            Ok(Self(slice))
        } else {
            Err(())
        }
    }

    pub fn center_chunk(&self) -> (i32, i32) {
        let ch = &self.0[Self::chunk_index(0, 0)];
        (ch.get_chunk_x(), ch.get_chunk_y())
    }

    pub fn pixel_to_chunk(x: i32, y: i32) -> (i8, i8) {
        ((x as f32 / CHUNK_SIZE as f32).floor() as i8, (y as f32 / CHUNK_SIZE as f32).floor() as i8)
    }

    pub fn chunk_index(cx: i8, cy: i8) -> usize {
        let center = S;
        let abs_x = (cx + center as i8) as usize;
        let abs_y = (cy + center as i8) as usize;
        let width = S * 2 + 1;
        abs_x + abs_y * width
    }

    pub fn set(&mut self, x: i32, y: i32, mat: MaterialInstance) -> Result<(), String> {
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        self.0[i].set(x.rem_euclid(CHUNK_SIZE as i32) as u16, y.rem_euclid(CHUNK_SIZE as i32) as u16, mat)
    }

    pub fn get(&self, x: i32, y: i32) -> Result<&MaterialInstance, String> {
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        self.0[i].get(x.rem_euclid(CHUNK_SIZE as i32) as u16, y.rem_euclid(CHUNK_SIZE as i32) as u16)
    }
}