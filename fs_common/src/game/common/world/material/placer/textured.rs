use image::{DynamicImage, GenericImageView};

use crate::game::common::world::material::{
    color::Color, MaterialID, MaterialInstance, PhysicsType,
};

use super::MaterialPlacer;

pub struct TexturedPlacer {
    material_id: MaterialID,
    physics: PhysicsType,
    image: DynamicImage,
}

impl TexturedPlacer {
    pub fn new(material_id: MaterialID, physics: PhysicsType, image_buf: &[u8]) -> Self {
        let image = image::load_from_memory(image_buf).unwrap();
        Self { material_id, physics, image }
    }
}

impl MaterialPlacer for TexturedPlacer {
    fn pixel(&self, x: i64, y: i64) -> MaterialInstance {
        let px = (x.rem_euclid(i64::from(self.image.width()))) as u32;
        let py = (y.rem_euclid(i64::from(self.image.height()))) as u32;

        // safety: the bounds are enforced with the `rem_euclid`s above
        let rgba = unsafe { self.image.unsafe_get_pixel(px, py) }.0;

        let color = Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]);

        MaterialInstance {
            material_id: self.material_id,
            physics: self.physics,
            color,
        }
    }
}
