use std::f32::consts::PI;

use fs_modding_api::{
    draw::RenderTarget,
    fs_common_types::{color::Color, modding::ModMeta, rect::Rect},
    fs_mod,
    util::get_time,
    Mod,
};

#[fs_mod]
#[derive(Default)]
struct TestMod {}

impl Mod for TestMod {
    fn meta(&self) -> ModMeta {
        ModMeta::new("test").with_display_name("Test Mod")
    }

    fn post_world_render(&self, draw_ctx: &mut RenderTarget) {
        let time = (get_time()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("duration_since")
            .as_millis()
            % 2000) as f32;
        draw_ctx.rectangle(
            Rect::new_wh(0.0, (time / 1000.0 * PI).sin() * 20.0, 20.0, 20.0),
            Color::CHARTREUSE_GREEN,
        );
    }
}
