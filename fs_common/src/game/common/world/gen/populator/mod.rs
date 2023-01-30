pub mod cave;
pub mod nearby_replace;
pub mod spawn;
pub mod test;

use crate::game::{
    common::world::{material::MaterialInstance, Chunk, CHUNK_SIZE},
    Registries,
};

// where S=0 means 1x1, S=1 means 3x3, etc
pub trait Populator<const S: u8> {
    fn populate(&self, chunks: &mut ChunkContext<S>, seed: i32, registries: &Registries);
}

// where S=0 means 1x1, S=1 means 3x3, etc
// when generic_const_exprs gets stablized eventually, could use [&mut dyn Chunk; (S * 2 + 1) * (S * 2 + 1)]
pub struct ChunkContext<'a, const S: u8>(&'a mut [&'a mut dyn Chunk]);

impl<'a, const S: u8> ChunkContext<'a, S> {
    pub fn new(slice: &'a mut [&'a mut dyn Chunk]) -> Result<Self, String> {
        if slice.len() == ((S * 2 + 1) * (S * 2 + 1)) as usize {
            Ok(Self(slice))
        } else {
            Err(format!(
                "Incorrect slice length, expected {}, got {}",
                (S * 2 + 1) * (S * 2 + 1),
                slice.len()
            ))
        }
    }

    pub fn center_chunk(&self) -> (i32, i32) {
        let ch = &self.0[Self::chunk_index(0, 0)];
        (ch.get_chunk_x(), ch.get_chunk_y())
    }

    pub fn pixel_to_chunk(x: i32, y: i32) -> (i8, i8) {
        (
            (x as f32 / f32::from(CHUNK_SIZE)).floor() as i8,
            (y as f32 / f32::from(CHUNK_SIZE)).floor() as i8,
        )
    }

    pub fn chunk_index(cx: i8, cy: i8) -> usize {
        let center = S;
        let abs_x = (i16::from(cx) + i16::from(center)) as usize;
        let abs_y = (i16::from(cy) + i16::from(center)) as usize;
        let width = S as usize * 2 + 1;
        abs_x + abs_y * width
    }

    pub fn set(&mut self, x: i32, y: i32, mat: MaterialInstance) -> Result<(), String> {
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        self.0[i].set(
            x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
            y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
            mat,
        )
    }

    pub fn get(&self, x: impl Into<i32>, y: impl Into<i32>) -> Result<&MaterialInstance, String> {
        let x = x.into();
        let y = y.into();
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        self.0[i].get(
            x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
            y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
        )
    }
}
