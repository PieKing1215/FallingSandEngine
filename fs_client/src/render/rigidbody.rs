use fs_common::game::common::world::rigidbody::FSRigidBody;
use glium::texture::Texture2d;

use super::drawing::RenderTarget;

pub trait FSRigidBodyExt {
    fn update_image(&mut self, target: &mut RenderTarget);
}

impl FSRigidBodyExt for FSRigidBody {
    fn update_image(&mut self, target: &mut RenderTarget) {
        if self.image_dirty {
            let pixel_data: Vec<_> = self
                .pixels
                .iter()
                .flat_map(|m| vec![m.color.r, m.color.g, m.color.b, m.color.a])
                .collect();

            let image = glium::texture::RawImage2d::from_raw_rgba(
                pixel_data,
                (self.width.into(), self.height.into()),
            );

            if self.image.is_none() {
                self.image = Some(Texture2d::new(&target.display, image).unwrap());
            } else {
                self.image.as_mut().unwrap().write(
                    glium::Rect {
                        left: 0,
                        bottom: 0,
                        width: self.width.into(),
                        height: self.height.into(),
                    },
                    image,
                );
            }

            self.image_dirty = false;
        }
    }
}
