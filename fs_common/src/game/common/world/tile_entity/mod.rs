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
    type D;

    #[allow(unused_variables)]
    fn tick(&mut self, common: &mut TileEntityCommon, ctx: TileEntityTickContext<Self::D>) {}
}

pub struct TileEntityTickContext<'a, D> {
    pub tick_time: u32,
    pub registries: Arc<Registries>,
    pub file_helper: &'a FileHelper,
    pub chunks: &'a mut [&'a mut chunksystem::Chunk<D>],
}

impl<S: TileEntitySided> TileEntity<S> {
    pub fn tick(&mut self, ctx: TileEntityTickContext<S::D>) {
        self.sided.tick(&mut self.common, ctx);
    }
}
