use std::fmt::Debug;

use super::{material::MaterialInstance, Chunk, ChunkHandler, ChunkHandlerGeneric, World};

#[derive(Clone)]
pub struct MaterialBuf {
    pub width: u16,
    pub height: u16,
    pub materials: Vec<MaterialInstance>,
}

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

    pub fn copy<C: Chunk>(
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
                buf.push(chunk_handler.get(wx, wy).copied()?);
            }
        }

        Ok(Self { width, height, materials: buf })
    }

    pub fn cut<C: Chunk>(
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
                buf.push(chunk_handler.get(wx, wy).copied()?);
                chunk_handler.set(wx, wy, MaterialInstance::air())?;
            }
        }

        Ok(Self { width, height, materials: buf })
    }

    pub fn paste<C: Chunk>(
        &self,
        chunk_handler: &mut ChunkHandler<C>,
        x: impl Into<i64>,
        y: impl Into<i64>,
    ) -> Result<(), String> {
        let x = x.into();
        let y = y.into();

        for dx in 0..self.width {
            for dy in 0..self.height {
                let wx = x + i64::from(dx);
                let wy = y + i64::from(dy);
                chunk_handler.set(
                    wx,
                    wy,
                    self.materials[dx as usize + dy as usize * self.width as usize],
                )?;
            }
        }

        Ok(())
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
