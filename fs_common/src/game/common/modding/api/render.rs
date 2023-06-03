use fs_common_types::{rect::Rect, color::Color};

use crate::game::common::modding::Mod;

pub trait PostWorldRenderTarget {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn rectangle(&mut self, rect: Rect<f32>, color: Color);
    fn rectangle_filled(&mut self, rect: Rect<f32>, color: Color);
}

impl Mod {
    pub fn post_world_render<T: PostWorldRenderTarget>(&mut self, target: &mut T) {
        self.call_ctx.with_post_world_render_target(target, |_| {
            self.plugin
                .call_function::<()>("post_world_render")
                .unwrap();
        });
    }
}
