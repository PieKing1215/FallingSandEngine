use std::f32::consts::PI;

use fs_mod_sdk::{
    fs_mod,
    fs_mod_common::{
        color::Color,
        modding::{render::RenderTarget, Mod, ModMeta},
        rect::Rect,
    },
    util::get_time,
};
use once_cell::sync::Lazy;

#[fs_mod]
#[derive(Default)]
struct TestMod {}

impl Mod for TestMod {
    fn meta(&self) -> &ModMeta {
        static META: Lazy<ModMeta> =
            Lazy::new(|| ModMeta::new("test").with_display_name("Test Mod"));
        &META
    }

    fn post_world_render(&mut self, draw_ctx: &mut dyn RenderTarget) {
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

    fn post_chunk_simulate(&mut self, colors: &mut [Color; 10000]) {
        for ele in colors {
            (ele.r, ele.g) = (ele.g, ele.r);
        }
    }
}
