pub mod bindings;
#[allow(non_snake_case)]
pub mod draw;
mod logging;
pub mod util;

use std::panic::PanicInfo;

use backtrace::Backtrace;
pub use fs_common_types;
pub use fs_modding_api_macros::*;
use log::LevelFilter;
use logging::FSModLogger;
pub use static_assertions;
pub use wasm_plugin_guest;

use draw::RenderTarget;
use fs_common_types::{color::Color, modding::ModMeta};
use once_cell::sync::OnceCell;

static INSTANCE: OnceCell<Box<dyn Mod + Send + Sync>> = OnceCell::new();
static LOGGER: FSModLogger = FSModLogger;

wasm_plugin_guest::import_functions! {
    fn panic(p: String);
}

pub fn init(inst: impl Mod + Send + Sync + 'static) -> ModMeta {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }

    std::panic::set_hook(Box::new(|p| {
        let bt = Backtrace::new();
        panic(format!("\n{p}\n{bt:?}\n"));
    }));

    let meta = inst.meta();

    if INSTANCE.set(Box::new(inst)).is_err() {
        log::error!("INSTANCE.set failed");
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
    fn post_chunk_simulate(&self, colors: &mut [Color; 10000]);
    // fn post_chunk_simulate(&self);
}
