mod manager;
pub use manager::*;

use fs_mod_common::{
    chunk::PostTickChunk,
    modding::{render::RenderTarget, ModMeta},
};
use std::{
    io::Cursor,
    sync::{Arc, RwLock},
};
use zip::ZipArchive;

use wasm_plugin_host::WasmPlugin;

use super::{
    asset_pack::AssetPack,
    dir_or_zip::DirOrZip,
    world::{material::color::Color, CHUNK_AREA},
};

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

pub struct Mod {
    meta: ModMeta,
    call_ctx: ModCallContext,
    plugin: WasmPlugin,
    root: DirOrZip,
}

impl Mod {
    pub fn load_asset_packs(&self) -> Vec<AssetPack> {
        self.root
            .iter_dir("asset_packs")
            .filter_map(|(mut f, _path)| {
                // TODO: this does not work for non-zip asset packs in mods
                if !f.is_dir() && f.extension() == Some("zip".to_string()) {
                    log::debug!("{:?}.{:?}", f.file_stem(), f.extension());
                    let zip_data = f.read().expect("Failed to read asset pack file");
                    let zip = ZipArchive::new(Box::new(Cursor::new(zip_data)) as _)
                        .expect("Failed to read asset pack zip");
                    Some(AssetPack::load(DirOrZip::Zip {
                        path: self.root.path().clone(),
                        zip: RwLock::new(zip),
                    }))
                } else {
                    None
                }
            })
            .filter_map(|res| match res {
                Ok(ap) => Some(ap),
                Err(err) => {
                    log::error!(
                        "Error loading mod asset pack from {:?}: {err:?}",
                        self.root.path()
                    );
                    None
                },
            })
            .collect()
    }
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
