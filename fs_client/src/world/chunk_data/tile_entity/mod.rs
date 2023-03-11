use core::slice;
use std::borrow::Cow;

use fs_common::game::common::world::{
    material::color::Color,
    tile_entity::{TileEntity, TileEntityCommon, TileEntitySided, TileEntityTickContext},
};
use glium::{Blend, DrawParameters, PolygonMode, Texture2d};

use crate::{render::drawing::RenderTarget, world::ClientChunk};

#[derive(Debug, Default)]
pub struct TileEntityClient {
    pub texture: Option<Texture2d>,
}

unsafe impl Send for TileEntityClient {}
unsafe impl Sync for TileEntityClient {}

impl TileEntitySided for TileEntityClient {
    type D = ClientChunk;

    fn tick(&mut self, common: &mut TileEntityCommon, ctx: TileEntityTickContext<Self::D>) {
        common
            .material_rect
            .translate(((ctx.tick_time as f32 / 15.0).cos() * 4.0) as _, 0);
    }
}

pub trait ClientTileEntityExt {
    fn render(&mut self, target: &mut RenderTarget);
}

impl ClientTileEntityExt for TileEntity<TileEntityClient> {
    fn render(&mut self, target: &mut RenderTarget) {
        let tex = self.sided.texture.get_or_insert_with(|| {
            let mut colors = self
                .common
                .material_rect
                .buf()
                .materials
                .iter()
                .map(|m| m.color)
                .collect::<Vec<_>>();

            let image = glium::texture::RawImage2d {
                data: Cow::Borrowed({
                    let sl = colors.as_mut_slice();
                    unsafe {
                        let sl: &mut [u8] =
                            slice::from_raw_parts_mut(sl.as_mut_ptr().cast(), sl.len() * 4);
                        sl
                    }
                }),
                width: self.common.material_rect.buf().width as _,
                height: self.common.material_rect.buf().height as _,
                format: glium::texture::ClientFormat::U8U8U8U8,
            };

            Texture2d::with_format(
                &target.display,
                image,
                glium::texture::UncompressedFloatFormat::U8U8U8U8,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap()
        });

        target.draw_texture_flipped(
            self.common.material_rect.rect().into_f32(),
            tex,
            DrawParameters {
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        );

        target.rectangle(
            self.common.material_rect.rect().into_f32(),
            Color::CYAN,
            DrawParameters {
                polygon_mode: PolygonMode::Line,
                line_width: Some(1.0),
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        );
    }
}
