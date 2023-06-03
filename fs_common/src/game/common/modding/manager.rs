use std::sync::Arc;

use fs_common_types::{color::Color, modding::ModMeta, rect::Rect};
use wasm_plugin_host::WasmPluginBuilder;

use crate::game::common::{
    modding::{CtxStorage, PostWorldRenderTarget},
    FileHelper,
};

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

        let call_ctx = ModCallContext { post_world_render_target: Arc::default() };

        for path in file_helper.mod_files() {
            log::info!("Loading mod {path:?}");

            let mut plugin = WasmPluginBuilder::from_file(&path)
                .expect("WasmPluginBuilder::from_file failed")
                .import_function("panic", |p: String| {
                    panic!("{p}");
                })
                .import_function("log_debug", |msg: String| {
                    log::debug!("{msg}");
                })
                .import_function("log_info", |msg: String| {
                    log::info!("{msg}");
                })
                .import_function("log_warn", |msg: String| {
                    log::warn!("{msg}");
                })
                .import_function("log_error", |msg: String| {
                    log::error!("{msg}");
                })
                .import_function("get_time", std::time::SystemTime::now)
                .import_function_with_context(
                    "RenderTarget_width",
                    call_ctx.post_world_render_target.clone(),
                    |rt: &CtxStorage<dyn PostWorldRenderTarget>| {
                        let rt = unsafe { &mut *rt.write().unwrap().as_mut().unwrap().value };

                        rt.width()
                    },
                )
                .import_function_with_context(
                    "RenderTarget_height",
                    call_ctx.post_world_render_target.clone(),
                    |rt: &CtxStorage<dyn PostWorldRenderTarget>| {
                        let rt = unsafe { &mut *rt.write().unwrap().as_mut().unwrap().value };

                        rt.height()
                    },
                )
                .import_function_with_context(
                    "RenderTarget_rectangle",
                    call_ctx.post_world_render_target.clone(),
                    |rt: &CtxStorage<dyn PostWorldRenderTarget>,
                     (rect, color): (Rect<f32>, Color)| {
                        let rt = unsafe { &mut *rt.write().unwrap().as_mut().unwrap().value };

                        rt.rectangle(rect, color);
                    },
                )
                .import_function_with_context(
                    "RenderTarget_rectangle_filled",
                    call_ctx.post_world_render_target.clone(),
                    |rt: &CtxStorage<dyn PostWorldRenderTarget>,
                     (rect, color): (Rect<f32>, Color)| {
                        let rt = unsafe { &mut *rt.write().unwrap().as_mut().unwrap().value };

                        rt.rectangle_filled(rect, color);
                    },
                )
                .finish()
                .expect("WasmPluginBuilder::finish failed");

            let meta = plugin.call_function::<ModMeta>("init").unwrap();

            log::info!("Initialized mod: {meta:?}");

            mods.push(Mod { meta, call_ctx: call_ctx.clone(), plugin });
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
