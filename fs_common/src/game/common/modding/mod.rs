mod manager;
pub mod api;
pub use manager::*;

use fs_common_types::{chunk::PostTickChunk, modding::ModMeta};
use std::{
    cell::UnsafeCell,
    sync::{Arc, RwLock},
};
use wasm_plugin_host::WasmPlugin;

use self::api::render::PostWorldRenderTarget;

use super::{
    world::{material::color::Color, CHUNK_AREA},
};

type CtxStorage<T> = Arc<RwLock<Option<SendSyncRawPtr<T>>>>;

struct SendSyncRawPtr<T: ?Sized> {
    pub value: *mut T,
}

unsafe impl Send for SendSyncRawPtr<dyn PostWorldRenderTarget> {}
unsafe impl Sync for SendSyncRawPtr<dyn PostWorldRenderTarget> {}

#[derive(Clone)]
pub struct ModCallContext {
    post_world_render_target: CtxStorage<dyn PostWorldRenderTarget>,
}

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

impl Mod {
    pub fn meta(&self) -> &ModMeta {
        &self.meta
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
