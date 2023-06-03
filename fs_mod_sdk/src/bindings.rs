use fs_common_types::chunk::PostTickChunk;
use wasm_plugin_guest::export_function;

use crate::{draw::RenderTarget, instance};

#[export_function]
pub fn post_world_render() {
    instance().post_world_render(&mut RenderTarget::new());
}

#[export_function]
pub fn post_chunk_simulate(colors: PostTickChunk) -> PostTickChunk {
    let mut colors = colors;
    instance().post_chunk_simulate(&mut colors.colors);
    colors
}

// #[export_function]
// pub fn post_chunk_simulate() {
//     instance().post_chunk_simulate();
// }
