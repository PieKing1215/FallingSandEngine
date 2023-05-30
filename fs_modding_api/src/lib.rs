pub mod bindings;
#[allow(non_snake_case)]
pub mod draw;
pub mod util;

pub use fs_common_types;
pub use fs_modding_api_macros::*;
pub use static_assertions;
pub use wasm_plugin_guest;

use draw::RenderTarget;
use fs_common_types::modding::ModMeta;
use once_cell::sync::OnceCell;

static INSTANCE: OnceCell<Box<dyn Mod + Send + Sync>> = OnceCell::new();

pub fn init(inst: impl Mod + Send + Sync + 'static) -> ModMeta {
    let meta = inst.meta();

    if INSTANCE.set(Box::new(inst)).is_err() {
        panic!("INSTANCE.set failed");
    }

    meta
}

fn instance() -> &'static (dyn Mod + Send + Sync) {
    INSTANCE.get().unwrap().as_ref()
}

#[allow(unused_variables)]
pub trait Mod {
    fn meta(&self) -> ModMeta;
    fn post_world_render(&self, draw_ctx: &mut RenderTarget) {}
}
