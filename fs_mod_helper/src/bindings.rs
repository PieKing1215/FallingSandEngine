use wasm_plugin_guest::export_function;

use crate::{draw::RenderTarget, instance};

#[export_function]
pub fn post_world_render() {
    instance().post_world_render(&mut RenderTarget::new());
}
