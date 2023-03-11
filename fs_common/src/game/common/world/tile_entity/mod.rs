use std::sync::Arc;

use crate::game::common::{FileHelper, Registries};

use super::material::buf::MaterialRect;

pub struct TileEntity<S> {
    pub common: TileEntityCommon,
    pub sided: S,
}

pub struct TileEntityCommon {
    pub material_rect: MaterialRect,
}

impl<S: Default> From<TileEntityCommon> for TileEntity<S> {
    fn from(common: TileEntityCommon) -> Self {
        Self { common, sided: S::default() }
    }
}

pub trait TileEntitySided {
    #[allow(unused_variables)]
    fn tick(&mut self, common: &mut TileEntityCommon, ctx: TileEntityTickContext) {}
}

pub struct TileEntityTickContext<'a> {
    pub tick_time: u32,
    pub registries: Arc<Registries>,
    pub file_helper: &'a FileHelper,
    // pub chunk_handler: &'a mut dyn ChunkHandlerGeneric, // TODO: need to refactor ChunkHandler completely for this to be possible
}

impl<S: TileEntitySided> TileEntity<S> {
    pub fn tick(&mut self, ctx: TileEntityTickContext) {
        self.sided.tick(&mut self.common, ctx);
    }
}
