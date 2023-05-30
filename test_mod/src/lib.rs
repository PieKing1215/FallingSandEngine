use std::f32::consts::PI;

use fs_modding_api::{
    draw::RenderTarget,
    fs_common_types::{color::Color, modding::ModMeta, rect::Rect},
    wasm_plugin_guest, Mod,
};
use wasm_plugin_guest::*;

#[export_function]
pub fn init() -> ModMeta {
    fs_modding_api::init(TestMod {})
}

struct TestMod {}

impl Mod for TestMod {
    fn meta(&self) -> ModMeta {
        ModMeta::new("test").with_display_name("Test Mod")
    }

    fn post_world_render(&self, draw_ctx: &mut RenderTarget) {
        draw_ctx.rectangle(
            Rect::new_wh(0.0, (0.0 / 1000.0 * PI).sin() * 20.0, 20.0, 20.0),
            Color::CHARTREUSE_GREEN,
        );
    }
}
