use fs_mod_common::chunk::PostTickChunk;
use wasm_plugin_guest::export_function;

use crate::{draw::DummyRT, instance_mut};

#[export_function]
pub fn post_world_render() {
    instance_mut().post_world_render(&mut DummyRT::new());
}

#[export_function]
pub fn post_chunk_simulate(colors: PostTickChunk) -> PostTickChunk {
    let mut colors = colors;
    instance_mut().post_chunk_simulate(&mut colors.colors);
    colors
}
