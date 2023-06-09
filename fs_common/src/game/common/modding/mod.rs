mod manager;
pub use manager::*;

use fs_mod_common::{
    chunk::PostTickChunk,
    modding::{render::RenderTarget, ModMeta},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use thiserror::Error;
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
        // SAFETY: `transmute` is only used to extend the lifetime of `t`, since we need to store it in `self` while calling `f`.
        // Since we had &mut of it and don't use it here while it is stored, I don't think this is UB.
        *self.post_world_render_target.write().unwrap() = Some(SendSyncRawPtr {
            value: unsafe { std::mem::transmute(t as *mut dyn RenderTarget) },
        });
        f(self);
        *self.post_world_render_target.write().unwrap() = None;
    }
}

pub enum ModRoot {
    Dir(PathBuf),
    Zip { path: PathBuf },
}

impl ModRoot {
    pub fn path(&self) -> &PathBuf {
        match self {
            ModRoot::Dir(path) | ModRoot::Zip { path } => path,
        }
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Option<Vec<u8>> {
        match self {
            ModRoot::Dir(root) => fs::read(root.join(path)).ok(),
            ModRoot::Zip { path: _ } => {
                // TODO
                todo!()
            },
        }
    }

    pub fn read_wasm(&self) -> Option<Vec<u8>> {
        self.read_file("mod.wasm")
    }
}

#[derive(Error, Debug)]
pub enum ModRootError {
    #[error("mod root not directory or zip")]
    NotDirOrZip(PathBuf),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

impl TryFrom<PathBuf> for ModRoot {
    type Error = ModRootError;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        if p.is_dir() {
            Ok(Self::Dir(p))
        } else if p
            .extension()
            .map_or(false, |ext| ext.eq_ignore_ascii_case("zip"))
        {
            Ok(ModRoot::Zip { path: p })
        } else {
            Err(ModRootError::NotDirOrZip(p))
        }
    }
}

pub struct Mod {
    meta: ModMeta,
    call_ctx: ModCallContext,
    plugin: WasmPlugin,
    root: ModRoot,
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
