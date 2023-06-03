mod manager;
pub use manager::*;

use fs_common_types::{chunk::PostTickChunk, modding::ModMeta};
use std::{
    cell::UnsafeCell,
    sync::{Arc, RwLock},
};
use wasm_plugin_host::WasmPlugin;

use super::{
    world::{material::color::Color, CHUNK_AREA},
    Rect,
};

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
