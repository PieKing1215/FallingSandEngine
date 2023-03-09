pub mod cave;
pub mod nearby_replace;
pub mod place_above;
pub mod spawn;
pub mod stalactite;
pub mod test;

use std::usize;

use crate::game::common::{
    world::{material::MaterialInstance, Chunk, CHUNK_SIZE},
    Registries,
};

// where S=0 means 1x1, S=1 means 3x3, etc
pub trait Populator<const S: u8> {
    fn populate(&self, chunks: &mut ChunkContext<S>, seed: i32, registries: &Registries);
}

// where S=0 means 1x1, S=1 means 3x3, etc
// when generic_const_exprs gets stablized eventually, could use [&mut dyn Chunk; (S * 2 + 1) * (S * 2 + 1)]
pub struct ChunkContext<'a, 'b, const S: u8>(&'a mut [&'b mut dyn Chunk]);

impl<'a, 'b, const S: u8> ChunkContext<'a, 'b, S> {
    pub fn new(slice: &'a mut [&'b mut dyn Chunk]) -> Result<Self, String> {
        if slice.len() == ((S * 2 + 1) * (S * 2 + 1)) as usize {
            if slice.iter().all(|c| c.pixels().is_some()) {
                Ok(Self(slice))
            } else {
                Err("Chunk was missing pixels".into())
            }
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
        (ch.chunk_x(), ch.chunk_y())
    }

    #[inline]
    pub fn pixel_to_chunk(x: i32, y: i32) -> (i8, i8) {
        (
            x.div_euclid(i32::from(CHUNK_SIZE)) as i8,
            y.div_euclid(i32::from(CHUNK_SIZE)) as i8,
        )
    }

    #[inline]
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
        // Safety: rem_euclid covers bounds check and we check in `Self::new` if the chunks have a pixel buffer
        unsafe {
            let ch = self.0.get_unchecked_mut(i);
            ch.set_unchecked(
                x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                mat,
            );
            Ok(())
        }
    }

    pub fn get(&self, x: impl Into<i32>, y: impl Into<i32>) -> Result<&MaterialInstance, String> {
        let x = x.into();
        let y = y.into();
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        // Safety: rem_euclid covers bounds check and we check in `Self::new` if the chunks have a pixel buffer
        unsafe {
            let ch = self.0.get_unchecked(i);
            Ok(ch.pixel_unchecked(
                x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
            ))
        }
    }

    fn replace<F>(&mut self, x: impl Into<i32>, y: impl Into<i32>, cb: F) -> bool
    where
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        let x = x.into();
        let y = y.into();

        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);

        let x = x.rem_euclid(i32::from(CHUNK_SIZE)) as u16;
        let y = y.rem_euclid(i32::from(CHUNK_SIZE)) as u16;
        unsafe {
            let ch = self.0.get_unchecked_mut(i);
            if let Some(mat) = (cb)(ch.pixel_unchecked(x, y)) {
                ch.set_unchecked(x, y, mat);
                true
            } else {
                false
            }
        }
    }

    pub fn set_background(&mut self, x: i32, y: i32, mat: MaterialInstance) -> Result<(), String> {
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        // Safety: rem_euclid covers bounds check and we check in `Self::new` if the chunks have a pixel buffer
        unsafe {
            let ch = self.0.get_unchecked_mut(i);
            ch.set_background_unchecked(
                x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                mat,
            );
            Ok(())
        }
    }

    pub fn get_background(
        &self,
        x: impl Into<i32>,
        y: impl Into<i32>,
    ) -> Result<&MaterialInstance, String> {
        let x = x.into();
        let y = y.into();
        let (cx, cy) = Self::pixel_to_chunk(x, y);
        let i = Self::chunk_index(cx, cy);
        // Safety: rem_euclid covers bounds check and we check in `Self::new` if the chunks have a pixel buffer
        unsafe {
            let ch = self.0.get_unchecked(i);
            Ok(ch.background_unchecked(
                x.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
                y.rem_euclid(i32::from(CHUNK_SIZE)) as u16,
            ))
        }
    }
}
