use std::fmt::Debug;

use super::{
    gen::structure::AngleMod,
    material::{self, MaterialInstance},
    Chunk, ChunkHandler, ChunkHandlerGeneric,
};

#[derive(Clone, PartialEq)]
pub struct MaterialBuf {
    pub width: u16,
    pub height: u16,
    pub materials: Vec<MaterialInstance>,
}

#[derive(Debug)]
pub struct OutOfBoundsError;

impl MaterialBuf {
    pub fn new(width: u16, height: u16, materials: Vec<MaterialInstance>) -> Result<Self, String> {
        if materials.len() == (width as usize * height as usize) {
            Ok(Self { width, height, materials })
        } else {
            Err(format!(
                "Incorrect materials Vec length, got {} expected {width}x{height}={}",
                materials.len(),
                (width as usize * height as usize)
            ))
        }
    }

    pub fn copy<C: Chunk + Send>(
        chunk_handler: &ChunkHandler<C>,
        x: impl Into<i64>,
        y: impl Into<i64>,
        width: impl Into<u16>,
        height: impl Into<u16>,
    ) -> Result<Self, String> {
        let x = x.into();
        let y = y.into();
        let width = width.into();
        let height = height.into();

        let mut buf = Vec::with_capacity(width as usize * height as usize);

        for dy in 0..height {
            for dx in 0..width {
                let wx = x + i64::from(dx);
                let wy = y + i64::from(dy);
                buf.push(chunk_handler.get(wx, wy).cloned()?);
            }
        }

        Ok(Self { width, height, materials: buf })
    }

    pub fn cut<C: Chunk + Send>(
        chunk_handler: &mut ChunkHandler<C>,
        x: impl Into<i64>,
        y: impl Into<i64>,
        width: impl Into<u16>,
        height: impl Into<u16>,
    ) -> Result<Self, String> {
        let x = x.into();
        let y = y.into();
        let width = width.into();
        let height = height.into();

        let mut buf = Vec::with_capacity(width as usize * height as usize);

        for dy in 0..height {
            for dx in 0..width {
                let wx = x + i64::from(dx);
                let wy = y + i64::from(dy);
                buf.push(chunk_handler.get(wx, wy).cloned()?);
                chunk_handler.set(wx, wy, MaterialInstance::air())?;
            }
        }

        Ok(Self { width, height, materials: buf })
    }

    pub fn paste(
        &self,
        chunk_handler: &mut dyn ChunkHandlerGeneric,
        x: impl Into<i64>,
        y: impl Into<i64>,
    ) -> Result<(), String> {
        let x = x.into();
        let y = y.into();

        for dx in 0..self.width {
            for dy in 0..self.height {
                let wx = x + i64::from(dx);
                let wy = y + i64::from(dy);
                let m = &self.materials[dx as usize + dy as usize * self.width as usize];
                if m.material_id != *material::STRUCTURE_VOID {
                    chunk_handler.set(wx, wy, m.clone())?;
                }
            }
        }

        Ok(())
    }

    pub fn get(&self, x: u16, y: u16) -> Result<MaterialInstance, OutOfBoundsError> {
        if x < self.width && y < self.height {
            Ok(self.materials[x as usize + y as usize * self.width as usize].clone())
        } else {
            Err(OutOfBoundsError)
        }
    }

    pub fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        if x < self.width && y < self.height {
            self.materials[x as usize + y as usize * self.width as usize] = mat;
        }
    }

    #[must_use]
    pub fn rotated(&self, angle: AngleMod) -> Self {
        let (new_w, new_h) = match angle {
            AngleMod::Clockwise90 | AngleMod::CounterClockwise90 => (self.height, self.width),
            _ => (self.width, self.height),
        };

        let mut new = Self {
            width: new_w,
            height: new_h,
            materials: vec![MaterialInstance::air(); new_w as usize * new_h as usize],
        };

        for new_x in 0..new_w {
            for new_y in 0..new_h {
                let (old_x, old_y) = match angle {
                    AngleMod::None => (new_x, new_y),
                    AngleMod::Clockwise90 => (new_y, new_w - new_x - 1),
                    AngleMod::CounterClockwise90 => (new_h - new_y - 1, new_x),
                    AngleMod::Angle180 => (new_w - new_x - 1, new_h - new_y - 1),
                };
                new.set(
                    new_x,
                    new_y,
                    self.get(old_x, old_y)
                        .expect(format!("{new_x} {new_y} {old_x} {old_y}").as_str()),
                );
            }
        }

        new
    }
}

impl Debug for MaterialBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialBuf")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}
