mod manager;
pub use manager::*;

use fs_mod_common::{
    chunk::PostTickChunk,
    modding::{render::RenderTarget, ModMeta},
};
use std::sync::{Arc, RwLock};
use wasm_plugin_host::WasmPlugin;

use super::world::{material::color::Color, CHUNK_AREA};

type CtxStorage<T> = Arc<RwLock<Option<SendSyncRawPtr<T>>>>;

struct SendSyncRawPtr<T: ?Sized> {
    pub value: *mut T,
}

unsafe impl Send for SendSyncRawPtr<dyn RenderTarget> {}
unsafe impl Sync for SendSyncRawPtr<dyn RenderTarget> {}

#[derive(Clone)]
pub struct ModCallContext {
    post_world_render_target: CtxStorage<dyn RenderTarget>,
}

impl ModCallContext {
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn with_post_world_render_target(
        &mut self,
        t: &mut dyn RenderTarget,
        f: impl FnOnce(&mut Self),
    ) {
        // TODO: this transmute could easily be UB, but I couldn't figure out any other way to do this
        // it's only being used to extend the lifetime of `t`, which will never be stored in `post_world_render_target` after this function returns
        *self.post_world_render_target.write().unwrap() =
            Some(unsafe { std::mem::transmute(t as *mut dyn RenderTarget) });
        f(self);
        *self.post_world_render_target.write().unwrap() = None;
    }
}

pub struct Mod {
    meta: ModMeta,
    call_ctx: ModCallContext,
    plugin: WasmPlugin,
}

impl fs_mod_common::modding::Mod for Mod {
    fn meta(&self) -> &ModMeta {
        &self.meta
    }

    fn post_world_render(&mut self, target: &mut dyn RenderTarget) {
        self.call_ctx.with_post_world_render_target(target, |_| {
            self.plugin
                .call_function::<()>("post_world_render")
                .unwrap();
        });
    }

    fn post_chunk_simulate(&mut self, colors: &mut [Color; CHUNK_AREA]) {
        profiling::scope!("post_chunk_simulate");

        let pt = PostTickChunk { colors: *colors };

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
            colors[i] = c;
        }
    }
}
