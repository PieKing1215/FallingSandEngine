use std::fmt::Debug;

use asefile::AsepriteFile;

use crate::game::common::{
    registry::RegistryID,
    world::{gen::structure::AngleMod, Chunk, ChunkHandler, ChunkHandlerGeneric},
    Rect,
};

use super::{color::Color, Material, MaterialInstance, PhysicsType};

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

    pub fn of_air(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            materials: vec![MaterialInstance::air(); (width * height) as usize],
        }
    }

    pub fn load_from_ase(ase: &AsepriteFile) -> Self {
        let w = ase.width() as u16;
        let h = ase.height() as u16;
        let mut buf = Self::new(w, h, vec![MaterialInstance::air(); (w * h) as usize]).unwrap();

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
                    override_color = Some(user.color.unwrap_or([0; 4].into()));
                }

                if let Some(l) = flags.iter().find(|f| f.starts_with("lit=")) {
                    let strength: f32 = l.trim_start_matches("lit=").parse().unwrap();
                    let c = user.color.unwrap();
                    light = [
                        f32::from(c.0[0]) / f32::from(u8::MAX) * strength,
                        f32::from(c.0[1]) / f32::from(u8::MAX) * strength,
                        f32::from(c.0[2]) / f32::from(u8::MAX) * strength,
                    ];
                } else if flags.contains(&"lit") {
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

        buf
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
                if m.material_id != *super::STRUCTURE_VOID {
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

pub struct MaterialRect {
    rect: Rect<i32>,
    buf: MaterialBuf,
}

#[derive(Debug)]
pub enum MaterialRectError {
    SizeMismatch,
}

impl MaterialRect {
    pub fn new(rect: Rect<i32>, buf: MaterialBuf) -> Result<Self, MaterialRectError> {
        if i32::from(buf.width) != rect.width() || i32::from(buf.height) != rect.height() {
            return Err(MaterialRectError::SizeMismatch)?;
        }

        Ok(Self { rect, buf })
    }

    pub fn new_air(rect: Rect<i32>) -> Self {
        Self {
            rect,
            buf: MaterialBuf::of_air(rect.width() as _, rect.height() as _),
        }
    }

    pub fn load_from_ase(ase: &AsepriteFile, top_left: (i32, i32)) -> Self {
        let buf = MaterialBuf::load_from_ase(ase);
        Self {
            rect: Rect::new_wh(top_left.0, top_left.1, buf.width, buf.height),
            buf,
        }
    }

    pub fn rect(&self) -> &Rect<i32> {
        &self.rect
    }

    pub fn buf(&self) -> &MaterialBuf {
        &self.buf
    }

    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.rect = Rect::new_wh(
            self.rect.x1 + dx,
            self.rect.y1 + dy,
            self.rect.width(),
            self.rect.height(),
        );
    }
}
