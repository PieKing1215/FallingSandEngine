use crate::game::common::Rect;

use super::{
    material::MaterialInstance, mesh::Mesh, tile_entity::TileEntity, ChunkLocalIndex,
    ChunkRigidBodyState, ChunkState, IndexLocal, CHUNK_AREA, CHUNK_SIZE,
};

pub struct CommonChunkData<S: SidedChunkData> {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<Box<[MaterialInstance; CHUNK_AREA]>>,
    pub light: Option<Box<[[f32; 3]; CHUNK_AREA]>>,
    pub background: Option<Box<[MaterialInstance; CHUNK_AREA]>>,
    pub dirty_rect: Option<Rect<i32>>,
    pub rigidbody: Option<ChunkRigidBodyState>,
    pub mesh_simplified: Option<Mesh>,
    pub tile_entities: Vec<TileEntity<S::TileEntityData>>,
}

pub trait SidedChunkData {
    type TileEntityData;
}

#[allow(clippy::missing_safety_doc)] // TODO
impl<S: SidedChunkData> CommonChunkData<S> {
    pub fn new(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            light: None,
            background: None,
            dirty_rect: None,
            rigidbody: None,
            mesh_simplified: None,
            tile_entities: vec![],
        }
    }

    pub fn set(
        &mut self,
        pos: impl Into<ChunkLocalIndex>,
        mat: MaterialInstance,
        mut cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(px) = &mut self.pixels {
            (cb)(&mat)?;

            *px.local_mut(pos) = mat;

            self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

            return Ok(());
        }

        Err("Chunk is not ready yet.".to_string())
    }

    pub unsafe fn set_unchecked(&mut self, pos: impl Into<ChunkLocalIndex>, mat: MaterialInstance) {
        *self.pixels.as_mut().unwrap_unchecked().local_mut(pos) = mat;

        self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));
    }

    pub fn pixel(&self, pos: impl Into<ChunkLocalIndex>) -> Result<&MaterialInstance, String> {
        if let Some(px) = &self.pixels {
            Ok(px.local(pos))
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub unsafe fn pixel_unchecked(&self, pos: impl Into<ChunkLocalIndex>) -> &MaterialInstance {
        self.pixels.as_ref().unwrap_unchecked().local(pos)
    }

    pub fn replace_pixel<F>(
        &mut self,
        pos: impl Into<ChunkLocalIndex>,
        cb: F,
        mut chunk_cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        if let Some(px) = &mut self.pixels {
            let i: ChunkLocalIndex = pos.into();
            let px = unsafe { px.get_unchecked_mut(*i) };
            if let Some(mat) = (cb)(px) {
                (chunk_cb)(&mat)?;
                *px = mat;

                self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(true);
            }

            Ok(false)
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub fn set_light(
        &mut self,
        pos: impl Into<ChunkLocalIndex>,
        light: [f32; 3],
        mut cb: impl FnMut(&[f32; 3]) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(li) = &mut self.light {
            (cb)(&light)?;

            *li.local_mut(pos) = light;

            // self.data.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

            Ok(())
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub unsafe fn set_light_unchecked(&mut self, pos: impl Into<ChunkLocalIndex>, light: [f32; 3]) {
        // TODO: should this unwrap be unchecked?
        *self.light.as_mut().unwrap().local_mut(pos) = light;
    }

    pub fn light(&self, pos: impl Into<ChunkLocalIndex>) -> Result<&[f32; 3], String> {
        if let Some(li) = &self.light {
            Ok(li.local(pos))
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub unsafe fn light_unchecked(&self, pos: impl Into<ChunkLocalIndex>) -> &[f32; 3] {
        // TODO: should this unwrap be unchecked?
        self.light.as_ref().unwrap().local(pos)
    }

    pub fn set_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>) {
        self.pixels = Some(pixels);
    }

    pub fn set_background(
        &mut self,
        pos: impl Into<ChunkLocalIndex>,
        mat: MaterialInstance,
        mut cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(px) = &mut self.background {
            (cb)(&mat)?;

            *px.local_mut(pos) = mat;

            Ok(())
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub unsafe fn set_background_unchecked(
        &mut self,
        pos: impl Into<ChunkLocalIndex>,
        mat: MaterialInstance,
    ) {
        // TODO: should this unwrap be unchecked?
        *self.background.as_mut().unwrap().local_mut(pos) = mat;
    }

    pub fn background(&self, pos: impl Into<ChunkLocalIndex>) -> Result<&MaterialInstance, String> {
        if let Some(px) = &self.background {
            Ok(px.local(pos))
        } else {
            Err("Chunk is not ready yet.".to_string())
        }
    }

    pub unsafe fn background_unchecked(
        &self,
        pos: impl Into<ChunkLocalIndex>,
    ) -> &MaterialInstance {
        // TODO: should this unwrap be unchecked?
        self.background.as_ref().unwrap().local(pos)
    }
}
