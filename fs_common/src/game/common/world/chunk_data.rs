use crate::game::common::Rect;

use super::{material::MaterialInstance, ChunkState, RigidBodyState, CHUNK_SIZE};

pub struct CommonChunkData {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub light: Option<Box<[[f32; 3]; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub background: Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub dirty_rect: Option<Rect<i32>>,
    pub rigidbody: Option<RigidBodyState>,
    pub mesh_simplified: Option<Vec<Vec<Vec<Vec<f64>>>>>,
}

#[allow(clippy::missing_safety_doc)] // TODO
impl CommonChunkData {
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
        }
    }

    pub fn set(
        &mut self,
        x: u16,
        y: u16,
        mat: MaterialInstance,
        mut cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                (cb)(&mat)?;
                *unsafe { px.get_unchecked_mut(i) } = mat;

                self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn set_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        *unsafe { self.pixels.as_mut().unwrap().get_unchecked_mut(i) } = mat;

        self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));
    }

    pub fn pixel(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { px.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn pixel_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.pixels.as_ref().unwrap().get_unchecked(i) }
    }

    pub fn replace_pixel<F>(
        &mut self,
        x: u16,
        y: u16,
        cb: F,
        mut chunk_cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                let px = unsafe { px.get_unchecked_mut(i) };
                if let Some(mat) = (cb)(px) {
                    (chunk_cb)(&mat)?;
                    *px = mat;

                    self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                    return Ok(true);
                }

                return Ok(false);
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub fn set_light(
        &mut self,
        x: u16,
        y: u16,
        light: [f32; 3],
        mut cb: impl FnMut(&[f32; 3]) -> Result<(), String>,
    ) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(li) = &mut self.light {
                (cb)(&light)?;
                // Safety: we do our own bounds check
                let i = (x + y * CHUNK_SIZE) as usize;
                *unsafe { li.get_unchecked_mut(i) } = light;

                // self.data.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn set_light_unchecked(&mut self, x: u16, y: u16, light: [f32; 3]) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        *unsafe { self.light.as_mut().unwrap().get_unchecked_mut(i) } = light;
    }

    pub fn light(&self, x: u16, y: u16) -> Result<&[f32; 3], String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(li) = &self.light {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { li.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn light_unchecked(&self, x: u16, y: u16) -> &[f32; 3] {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.light.as_ref().unwrap().get_unchecked(i) }
    }

    pub fn set_pixels(
        &mut self,
        pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    ) {
        self.pixels = Some(pixels);
        self.light = Some(Box::new([[0.0; 3]; (CHUNK_SIZE * CHUNK_SIZE) as usize]));
    }

    pub fn set_background(
        &mut self,
        x: u16,
        y: u16,
        mat: MaterialInstance,
        mut cb: impl FnMut(&MaterialInstance) -> Result<(), String>,
    ) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.background {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                (cb)(&mat)?;
                *unsafe { px.get_unchecked_mut(i) } = mat;

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn set_background_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        *unsafe { self.background.as_mut().unwrap().get_unchecked_mut(i) } = mat;
    }

    pub fn background(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &self.background {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { px.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    pub unsafe fn background_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.background.as_ref().unwrap().get_unchecked(i) }
    }
}
