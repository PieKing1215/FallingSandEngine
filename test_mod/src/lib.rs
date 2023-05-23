use std::f32::consts::PI;

use extism_pdk::*;

#[plugin_fn]
pub fn test(_: ()) -> FnResult<String> {
    Ok("Hello test_fn!".into())
}

#[plugin_fn]
pub fn post_world_render(_: ()) -> FnResult<()> {
    let time = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("duration_since")
        .as_millis()
        % 2000) as f32;
    unsafe { draw_rect(0.0, (time / 1000.0 * PI).sin() * 20.0, 20.0, 20.0) };
    Ok(())
}

extern "C" {
    fn draw_rect(x: f32, y: f32, w: f32, h: f32);
}
