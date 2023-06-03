use fs_common_types::{chunk::PostTickChunk, modding::ModMeta};
use std::{
    cell::UnsafeCell,
    sync::{Arc, Mutex, RwLock},
};
use wasm_plugin_host::{WasmPlugin, WasmPluginBuilder};

use super::{
    world::{material::color::Color, CHUNK_AREA},
    FileHelper, Rect,
};

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

type CtxStorage<T> = Arc<RwLock<Option<UnsafeSendSync<*mut T>>>>;

#[derive(Clone)]
pub struct ModCallContext {
    post_world_render_target: CtxStorage<dyn PostWorldRenderTarget>,
}

struct UnsafeSendSync<T> {
    pub value: T,
}

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

impl ModCallContext {
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn with_post_world_render_target<T: PostWorldRenderTarget>(
        &mut self,
        t: &mut T,
        f: impl FnOnce(&mut Self),
    ) {
        // TODO: this transmute could easily be UB, but I couldn't figure out any other way to do this
        // it's only being used to extend the lifetime of `t`, which will never be stored in `post_world_render_target` after this function returns
        *self.post_world_render_target.write().unwrap() =
            Some(unsafe { std::mem::transmute(t as *mut dyn PostWorldRenderTarget) });
        f(self);
        *self.post_world_render_target.write().unwrap() = None;
    }
}

pub struct Mod {
    meta: ModMeta,
    call_ctx: ModCallContext,
    plugin: WasmPlugin,
}

pub trait PostWorldRenderTarget {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn rectangle(&mut self, rect: Rect<f32>, color: Color);
    fn rectangle_filled(&mut self, rect: Rect<f32>, color: Color);
}

impl Mod {
    pub fn meta(&self) -> &ModMeta {
        &self.meta
    }

    pub fn post_world_render<T: PostWorldRenderTarget>(&mut self, target: &mut T) {
        self.call_ctx.with_post_world_render_target(target, |_| {
            self.plugin
                .call_function::<()>("post_world_render")
                .unwrap();
        });
    }

    pub fn post_chunk_simulate(&mut self, colors: &[UnsafeCell<Color>; CHUNK_AREA]) {
        profiling::scope!("post_chunk_simulate");

        let in_colors = colors
            .iter()
            .map(|uc| unsafe { *uc.get() })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let pt = PostTickChunk { colors: in_colors };

        let res = {
            profiling::scope!("call_function_with_argument");
            self.plugin
                .call_function_with_argument::<PostTickChunk, PostTickChunk>(
                    "post_chunk_simulate",
                    &pt,
                )
                .unwrap()
        };

        for (i, c) in res.colors.into_iter().enumerate() {
            unsafe { *colors[i].get() = c };
        }
    }
}