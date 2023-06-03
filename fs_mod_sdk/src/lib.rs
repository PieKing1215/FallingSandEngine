pub mod bindings;
#[allow(non_snake_case)]
pub mod draw;
mod logging;
pub mod util;

use backtrace::Backtrace;
pub use fs_mod_common;
pub use fs_mod_sdk_macros::*;
use log::LevelFilter;
use logging::FSModLogger;
pub use static_assertions;
pub use wasm_plugin_guest;

use fs_mod_common::modding::{Mod, ModMeta};
use once_cell::sync::OnceCell;

static mut INSTANCE: OnceCell<Box<dyn Mod + Send + Sync>> = OnceCell::new();
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

    let meta = inst.meta().clone();

    if unsafe { INSTANCE.set(Box::new(inst)) }.is_err() {
        log::error!("INSTANCE.set failed");
        panic!("INSTANCE.set failed");
    }

    meta
}

#[allow(dead_code)]
fn instance() -> &'static (dyn Mod + Send + Sync) {
    unsafe { INSTANCE.get() }.unwrap().as_ref()
}

fn instance_mut() -> &'static mut (dyn Mod + Send + Sync) {
    unsafe { INSTANCE.get_mut() }.unwrap().as_mut()
}
