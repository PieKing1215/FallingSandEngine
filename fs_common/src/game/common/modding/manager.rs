use std::sync::Arc;

use fs_mod_common::{
    color::Color,
    modding::{render::RenderTarget, ModMeta},
    rect::Rect,
};
use wasm_plugin_host::WasmPluginBuilder;

use crate::game::common::{modding::CtxStorage, FileHelper};

use super::{Mod, ModCallContext};

pub struct ModManager {
    mods: Vec<Mod>,
}

impl ModManager {
    pub fn empty() -> Self {
        Self { mods: vec![] }
    }

    pub fn init(file_helper: &FileHelper) -> Self {
        let mut mods = vec![];

        for root in file_helper.mod_files() {
            log::info!("Loading mod {:?}", root.path());

            let wasm = root
                .read_file("mod.wasm")
                .expect("Mod missing mod.wasm file");

            let builder = WasmPluginBuilder::from_source(&wasm)
                .expect("WasmPluginBuilder::from_source failed");
            let (builder, call_ctx) = register_fns(builder);

            let mut plugin = builder.finish().expect("WasmPluginBuilder::finish failed");

            let meta = plugin.call_function::<ModMeta>("init").unwrap();

            log::info!("Initialized mod: {meta:?}");

            mods.push(Mod { meta, call_ctx: call_ctx.clone(), plugin, root });
        }

        Self { mods }
    }

    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    pub fn mods_mut(&mut self) -> &mut [Mod] {
        &mut self.mods
    }
}

fn register_fns(mut builder: WasmPluginBuilder) -> (WasmPluginBuilder, ModCallContext) {
    let call_ctx = ModCallContext { post_world_render_target: Arc::default() };

    macro_rules! import {
        ($(fn $i:ident($( $pi:ident: $t:ty )*) $(-> $ret:ty)? $b:block)*) => {
            $(builder = builder.import_function(stringify!($i), |$($pi: $t)*| $b);)*
        };
    }

    macro_rules! import_ctx {
        ($get_ctx:expr => $ctx_ty:ty; $(fn $i:ident($ctx_v:ident $($(, $pi:ident: $t:ty )+)?) $(-> $ret:ty)? $b:block)*) => {
            $(builder = builder.import_function_with_context(
                stringify!($i),
                $get_ctx,
                |$ctx_v: &CtxStorage<$ctx_ty>$(, ($($pi,)+) : ($($t,)+))?| {
                    let $ctx_v = unsafe { &mut *$ctx_v.write().unwrap().as_mut().unwrap().value };
                    $b
                },
            );)*
        };
    }

    import! {
        fn panic(p: String) {
            panic!("{p}");
        }

        fn log_debug(msg: String) {
            log::debug!("{msg}");
        }

        fn log_info(msg: String) {
            log::info!("{msg}");
        }

        fn log_warn(msg: String) {
            log::warn!("{msg}");
        }

        fn log_error(msg: String) {
            log::error!("{msg}");
        }

        fn get_time() -> std::time::SystemTime {
            std::time::SystemTime::now()
        }
    }

    import_ctx! {
        call_ctx.post_world_render_target.clone() => dyn RenderTarget;

        fn RenderTarget_width(rt) -> u32 {
            rt.width()
        }

        fn RenderTarget_height(rt) -> u32 {
            rt.height()
        }

        fn RenderTarget_rectangle(rt, rect: Rect<f32>, color: Color) {
            rt.rectangle(rect, color);
        }

        fn RenderTarget_rectangle_filled(rt, rect: Rect<f32>, color: Color) {
            rt.rectangle_filled(rect, color);
        }
    }

    (builder, call_ctx)
}
