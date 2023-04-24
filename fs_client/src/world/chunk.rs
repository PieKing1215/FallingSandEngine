use chunksystem::ChunkQuery;
use core::slice;
use fs_common::game::common::world::{
    material::PhysicsType, Chunk, ChunkLocalIndex, ChunkLocalPosition, CHUNK_AREA,
};
use std::{borrow::Cow, convert::TryInto, sync::Arc};

use fs_common::game::common::{
    world::{
        chunk_data::{CommonChunkData, SidedChunkData},
        material::{color::Color, MaterialInstance},
        mesh::{self, Mesh},
        tile_entity::{TileEntity, TileEntityCommon},
        ChunkHandler, ChunkRigidBodyState, ChunkState, SidedChunk, CHUNK_SIZE, LIGHT_SCALE,
    },
    FileHelper, Rect, Settings,
};
use glium::{
    pixel_buffer::PixelBuffer, texture::Texture2d, uniform, uniforms::ImageUnit, Blend, Display,
    DrawParameters, PolygonMode,
};

use crate::render::{drawing::RenderTarget, shaders::Shaders};

use super::chunk_data::tile_entity::TileEntityClient;

pub struct ClientChunk {
    pub data: CommonChunkData<Self>,
    pub graphics: Box<ChunkGraphics>,
    pub mesh: Option<Mesh>,
    pub tris: Option<Vec<Vec<mesh::Tri>>>,
}

impl SidedChunkData for ClientChunk {
    type TileEntityData = TileEntityClient;
}

impl Chunk for ClientChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            data: CommonChunkData::new(chunk_x, chunk_y),
            graphics: Box::new(ChunkGraphics {
                data: None,
                pixel_data: Box::new([Color::TRANSPARENT; CHUNK_AREA]),
                lighting_data: Box::new([[0.0; 4]; CHUNK_AREA]),
                background_data: Box::new([Color::TRANSPARENT; CHUNK_AREA]),
                dirty: true,
                was_dirty: true,
                lighting_dirty: true,
                was_lighting_dirty: true,
                background_dirty: true,
                pixels_updated_last_update: true,
                lighting_updated_last_update: true,
                dist_to_nearest_dirty_light: None,
                prev_dist_to_nearest_dirty_light: None,
            }),
            mesh: None,
            tris: None,
        }
    }

    fn chunk_x(&self) -> i32 {
        self.data.chunk_x
    }

    fn chunk_y(&self) -> i32 {
        self.data.chunk_y
    }

    fn state(&self) -> ChunkState {
        self.data.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.data.state = state;
    }

    fn dirty_rect(&self) -> Option<Rect<i32>> {
        self.data.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>) {
        self.data.dirty_rect = rect;
    }

    fn refresh(&mut self) {
        for pos in ChunkLocalPosition::iter() {
            let i: ChunkLocalIndex = pos.into();
            self.graphics
                .set(pos, self.data.pixels.as_ref().unwrap()[i].color);
            self.graphics
                .set_light(pos, self.data.light.as_ref().unwrap()[i]);
        }
    }

    // #[profiling::function] // huge performance impact
    fn set_pixel(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) -> Result<(), String> {
        self.data.set(pos, mat, |mat| {
            if mat.physics != PhysicsType::Object {
                self.graphics.set(pos, mat.color);
                self.graphics.set_light(pos, mat.light);
            }

            Ok(())
        })
    }

    unsafe fn set_pixel_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) {
        if mat.physics != PhysicsType::Object {
            self.graphics.set(pos, mat.color);
            self.graphics.set_light(pos, mat.light);
        }

        self.data.set_unchecked(pos, mat);
    }

    fn pixel(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String> {
        self.data.pixel(pos)
    }

    unsafe fn pixel_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance {
        self.data.pixel_unchecked(pos)
    }

    fn replace_pixel<F>(&mut self, pos: ChunkLocalPosition, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        self.data.replace_pixel(pos, cb, |m| {
            if m.physics != PhysicsType::Object {
                self.graphics.set(pos, m.color);
                self.graphics.set_light(pos, m.light);
            }

            Ok(())
        })
    }

    fn set_light(&mut self, pos: ChunkLocalPosition, light: [f32; 3]) -> Result<(), String> {
        self.data.set_light(pos, light, |l| {
            self.graphics.set_light(pos, *l);
            Ok(())
        })
    }

    unsafe fn set_light_unchecked(&mut self, pos: ChunkLocalPosition, light: [f32; 3]) {
        self.graphics.set_light(pos, light);

        self.data.set_light_unchecked(pos, light);
    }

    fn light(&self, pos: ChunkLocalPosition) -> Result<&[f32; 3], String> {
        self.data.light(pos)
    }

    unsafe fn light_unchecked(&self, pos: ChunkLocalPosition) -> &[f32; 3] {
        self.data.light_unchecked(pos)
    }

    fn set_color(&mut self, pos: ChunkLocalPosition, color: Color) {
        self.graphics.set(pos, color);
    }

    fn color(&self, pos: ChunkLocalPosition) -> Color {
        self.graphics.get(pos)
    }

    #[profiling::function]
    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>) {
        self.data.set_pixels(pixels);
    }

    fn pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &mut self.data.pixels
    }

    fn pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &self.data.pixels
    }

    #[profiling::function]
    fn set_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.graphics.replace(colors);
    }

    fn colors_mut(&mut self) -> &mut [Color; CHUNK_AREA] {
        &mut self.graphics.pixel_data
    }

    fn colors(&self) -> &[Color; CHUNK_AREA] {
        &self.graphics.pixel_data
    }

    #[profiling::function]
    fn set_background_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>) {
        self.data.background = Some(pixels);
    }

    fn background_pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &mut self.data.background
    }

    fn background_pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &self.data.background
    }

    #[profiling::function]
    fn set_background_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.graphics.replace_background(colors);
    }

    fn background_colors_mut(&mut self) -> &mut [Color; CHUNK_AREA] {
        &mut self.graphics.background_data
    }

    fn background_colors(&self) -> &[Color; CHUNK_AREA] {
        &self.graphics.background_data
    }

    fn mark_dirty(&mut self) {
        self.graphics.dirty = true;
        self.graphics.background_dirty = true;
        self.graphics.lighting_dirty = true;
    }

    fn generate_mesh(&mut self) -> Result<(), String> {
        if self.data.pixels.is_none() {
            return Err("generate_mesh failed: self.data.pixels is None".to_owned());
        }

        let vs: Vec<f64> = mesh::pixels_to_valuemap(self.data.pixels.as_ref().unwrap().as_ref());

        let generated =
            mesh::generate_mesh_with_simplified(&vs, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));

        if let Ok(r) = generated {
            self.mesh = Some(r.0);
            self.data.mesh_simplified = Some(r.1);
        } else {
            self.mesh = None;
            self.data.mesh_simplified = None;
        }

        self.tris = self.data.mesh_simplified.as_ref().map(mesh::triangulate);

        Ok(())
    }

    fn mesh_loops(&self) -> &Option<Mesh> {
        &self.data.mesh_simplified
    }

    fn rigidbody(&self) -> &Option<ChunkRigidBodyState> {
        &self.data.rigidbody
    }

    fn rigidbody_mut(&mut self) -> &mut Option<ChunkRigidBodyState> {
        &mut self.data.rigidbody
    }

    fn set_rigidbody(&mut self, body: Option<ChunkRigidBodyState>) {
        self.data.rigidbody = body;
    }

    fn lights_mut(&mut self) -> &mut [[f32; 4]; CHUNK_AREA] {
        &mut self.graphics.lighting_data
    }

    fn lights(&self) -> &[[f32; 4]; CHUNK_AREA] {
        &self.graphics.lighting_data
    }

    fn set_background(
        &mut self,
        pos: ChunkLocalPosition,
        mat: MaterialInstance,
    ) -> Result<(), String> {
        self.data.set_background(pos, mat, |m| {
            self.graphics.set_background(pos, m.color);
            Ok(())
        })
    }

    unsafe fn set_background_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) {
        self.graphics.set_background(pos, mat.color);
        self.data.set_background_unchecked(pos, mat);
    }

    fn background(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String> {
        self.data.background(pos)
    }

    unsafe fn background_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance {
        self.data.background_unchecked(pos)
    }

    fn add_tile_entity(&mut self, te: TileEntityCommon) {
        self.data.tile_entities.push(te.into());
    }

    fn common_tile_entities(&self) -> Box<dyn Iterator<Item = &TileEntityCommon> + '_> {
        Box::new(self.data.tile_entities.iter().map(|te| &te.common))
    }

    fn common_tile_entities_mut(&mut self) -> Box<dyn Iterator<Item = &mut TileEntityCommon> + '_> {
        Box::new(self.data.tile_entities.iter_mut().map(|te| &mut te.common))
    }
}

impl SidedChunk for ClientChunk {
    type S = Self;

    fn sided_tile_entities(&self) -> &[TileEntity<<Self::S as SidedChunkData>::TileEntityData>] {
        &self.data.tile_entities
    }

    fn sided_tile_entities_mut(
        &mut self,
    ) -> &mut [TileEntity<<Self::S as SidedChunkData>::TileEntityData>] {
        &mut self.data.tile_entities
    }

    fn sided_tile_entities_removable(
        &mut self,
    ) -> &mut Vec<TileEntity<<Self::S as SidedChunkData>::TileEntityData>> {
        &mut self.data.tile_entities
    }
}

pub struct ChunkGraphicsData {
    pub display: Display,
    pub texture: Texture2d,
    pub background_texture: Texture2d,
    pub lighting_src_buf: PixelBuffer<(f32, f32, f32, f32)>,
    pub lighting_src: Texture2d,
    pub lighting_dst: Texture2d,
    pub lighting_neighbors: Texture2d,
    pub lighting_constant_black: Texture2d,
}

pub struct ChunkGraphics {
    pub data: Option<Arc<ChunkGraphicsData>>,
    pub pixel_data: Box<[Color; CHUNK_AREA]>,
    pub lighting_data: Box<[[f32; 4]; CHUNK_AREA]>,
    pub background_data: Box<[Color; CHUNK_AREA]>,
    pub dirty: bool,
    pub was_dirty: bool,
    pub lighting_dirty: bool,
    pub was_lighting_dirty: bool,
    pub background_dirty: bool,

    pub pixels_updated_last_update: bool,
    pub lighting_updated_last_update: bool,

    pub prev_dist_to_nearest_dirty_light: Option<u8>,
    pub dist_to_nearest_dirty_light: Option<u8>,
}

unsafe impl Send for ChunkGraphics {}
unsafe impl Sync for ChunkGraphics {}

impl ChunkGraphics {
    // #[profiling::function] // huge performance impact
    pub fn set(&mut self, pos: impl Into<ChunkLocalIndex>, color: Color) {
        let i: ChunkLocalIndex = pos.into();
        if self.pixel_data[i] != color {
            if self.pixel_data[i].a != color.a {
                self.lighting_dirty = true;
            }
            self.pixel_data[i] = color;
            self.dirty = true;
        }
    }

    // #[profiling::function] // huge performance impact
    pub fn set_light(&mut self, pos: impl Into<ChunkLocalIndex>, color: [f32; 3]) {
        let i: ChunkLocalIndex = pos.into();
        if self.lighting_data[i] != [color[0], color[1], color[2], 1.0] {
            self.lighting_data[i] = [color[0], color[1], color[2], 1.0];
            self.lighting_dirty = true;
        }
    }

    // #[profiling::function] // huge performance impact
    pub fn get(&self, pos: impl Into<ChunkLocalIndex>) -> Color {
        let i: ChunkLocalIndex = pos.into();
        self.pixel_data[i]
    }

    pub fn set_background(&mut self, pos: impl Into<ChunkLocalIndex>, color: Color) {
        let i: ChunkLocalIndex = pos.into();
        if self.background_data[i] != color {
            self.background_data[i] = color;
            self.background_dirty = true;
        }
    }

    // #[profiling::function]
    pub fn update_texture(&mut self) {
        self.pixels_updated_last_update = false;
        if self.dirty {
            if let Some(data) = &mut self.data {
                profiling::scope!("dirty");

                let image = {
                    profiling::scope!("RawImage2d");

                    glium::texture::RawImage2d {
                        data: Cow::Borrowed({
                            let color_sl = self.pixel_data.as_slice();
                            unsafe {
                                // Safety: Color is statically guaranteed to be equivalent to four u8s
                                core::slice::from_raw_parts(
                                    color_sl.as_ptr().cast::<u8>(),
                                    color_sl.len() * 4,
                                )
                            }
                        }),
                        width: CHUNK_SIZE.into(),
                        height: CHUNK_SIZE.into(),
                        format: glium::texture::ClientFormat::U8U8U8U8,
                    }
                };

                {
                    profiling::scope!("write");
                    data.texture.write(
                        glium::Rect {
                            left: 0,
                            bottom: 0,
                            width: CHUNK_SIZE.into(),
                            height: CHUNK_SIZE.into(),
                        },
                        image,
                    );
                }

                self.pixels_updated_last_update = true;
                self.dirty = false;
            }
        }

        if self.background_dirty {
            if let Some(data) = &mut self.data {
                profiling::scope!("background_dirty");
                let image = {
                    profiling::scope!("RawImage2d");

                    glium::texture::RawImage2d {
                        data: Cow::Borrowed({
                            let color_sl = self.background_data.as_slice();
                            unsafe {
                                // Safety: Color is statically guaranteed to be equivalent to four u8s
                                core::slice::from_raw_parts(
                                    color_sl.as_ptr().cast::<u8>(),
                                    color_sl.len() * 4,
                                )
                            }
                        }),
                        width: CHUNK_SIZE.into(),
                        height: CHUNK_SIZE.into(),
                        format: glium::texture::ClientFormat::U8U8U8U8,
                    }
                };

                {
                    profiling::scope!("write");
                    data.background_texture.write(
                        glium::Rect {
                            left: 0,
                            bottom: 0,
                            width: CHUNK_SIZE.into(),
                            height: CHUNK_SIZE.into(),
                        },
                        image,
                    );
                }
                self.background_dirty = false;
            }
        }
    }

    // #[profiling::function]
    pub fn update_lighting(
        &mut self,
        neighbors: Option<[Option<&chunksystem::Chunk<ClientChunk>>; 4]>,
        shaders: &Shaders,
    ) {
        self.lighting_updated_last_update = false;
        if self.lighting_dirty || self.dist_to_nearest_dirty_light.is_some() {
            if let Some(data) = &mut self.data {
                profiling::scope!("lighting update");

                let src_image = {
                    profiling::scope!("src RawImage2d");
                    glium::texture::RawImage2d {
                        data: Cow::Borrowed({
                            profiling::scope!("format data");
                            // Safety: transmuting &[[f32; 4]] to &[f32] should be fine since arrays are contiguous
                            // TODO: use `self.lighting_data.flatten()` once stabilized (https://github.com/rust-lang/rust/issues/95629)
                            let sl: &[f32] = unsafe {
                                slice::from_raw_parts(
                                    self.lighting_data.as_ptr().cast(),
                                    self.lighting_data.len() * 4,
                                )
                            };
                            sl
                        }),
                        width: CHUNK_SIZE.into(),
                        height: CHUNK_SIZE.into(),
                        format: glium::texture::ClientFormat::F32F32F32F32,
                    }
                };

                {
                    profiling::scope!("src write");
                    data.lighting_src.write(
                        glium::Rect {
                            left: 0,
                            bottom: 0,
                            width: CHUNK_SIZE.into(),
                            height: CHUNK_SIZE.into(),
                        },
                        src_image,
                    );
                }

                fn r32f_read(tex: &Texture2d) -> ImageUnit<Texture2d> {
                    tex.image_unit(glium::uniforms::ImageUnitFormat::RGBA32F)
                        .unwrap()
                        .set_access(glium::uniforms::ImageUnitAccess::Read)
                }

                let t_src = r32f_read(&data.lighting_src);
                let t_px = data
                    .texture
                    .image_unit(glium::uniforms::ImageUnitFormat::RGBA8)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::Read);
                let t_dst = data
                    .lighting_dst
                    .image_unit(glium::uniforms::ImageUnitFormat::RGBA32F)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::Write);
                let t_work = data
                    .lighting_neighbors
                    .image_unit(glium::uniforms::ImageUnitFormat::RGBA32F)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::ReadWrite);

                let t_light_n = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[0].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_w = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[1].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_e = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[2].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_s = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[3].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );

                let uni = uniform! {
                    light_scale: LIGHT_SCALE as i32,
                    t_src: t_src,
                    t_light_n: t_light_n,
                    t_light_e: t_light_e,
                    t_light_s: t_light_s,
                    t_light_w: t_light_w,
                    t_work: t_work,
                };

                {
                    profiling::scope!("prep");
                    shaders.lighting_compute_prep.execute(uni, 1, 1, 1);
                }

                let t_work = data
                    .lighting_neighbors
                    .image_unit(glium::uniforms::ImageUnitFormat::RGBA32F)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::ReadWrite);

                let uni = uniform! {
                    light_scale: LIGHT_SCALE as i32,
                    t_px: t_px,
                    t_dst: t_dst,
                    t_work: t_work,
                };

                {
                    profiling::scope!("propagate");
                    shaders.lighting_compute_propagate.execute(uni, 1, 1, 1);
                }

                if self.lighting_dirty {
                    self.dist_to_nearest_dirty_light = Some(0);
                }

                self.lighting_updated_last_update = true;
                self.lighting_dirty = false;
            }
        }
    }

    #[profiling::function]
    #[allow(clippy::cast_lossless)]
    pub fn replace(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.pixel_data = colors;
        self.dirty = true;
    }

    #[profiling::function]
    #[allow(clippy::cast_lossless)]
    pub fn replace_background(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.background_data = colors;
        self.background_dirty = true;
    }
}

impl ClientChunk {
    #[profiling::function]
    fn update_graphics(
        &mut self,
        surrounding: Option<[Option<&chunksystem::Chunk<Self>>; 4]>,
        shaders: &Shaders,
    ) -> Result<(), String> {
        self.graphics.update_texture();
        self.graphics.update_lighting(surrounding, shaders);

        Ok(())
    }

    pub fn prep_render(
        &mut self,
        target: &mut RenderTarget,
        settings: &Settings,
        file_helper: &FileHelper,
    ) {
        self.graphics.prep_render(target, settings, file_helper);
    }

    pub fn render(&mut self, target: &mut RenderTarget, settings: &Settings) {
        if settings.debug && settings.draw_chunk_collision == 1 {
            if let Some(f) = &self.mesh {
                let colors = vec![
                    Color::rgb(32, 255, 32),
                    Color::rgb(255, 32, 32),
                    Color::rgb(32, 64, 255),
                    Color::rgb(255, 255, 32),
                    Color::rgb(32, 255, 255),
                    Color::rgb(255, 32, 255),
                ];

                let lines = f
                    .iter()
                    .enumerate()
                    .flat_map(|(j, f)| {
                        let c = colors[j % colors.len()];
                        f.iter().flat_map(move |pts| {
                            let mut v = vec![];
                            for i in 1..pts.len() {
                                let (x1, y1) = (pts[i - 1][0], pts[i - 1][1]);
                                let (x2, y2) = (pts[i][0], pts[i][1]);
                                v.push(((x1 as f32, y1 as f32), (x2 as f32, y2 as f32), (c)));
                            }
                            v

                            // draw individual points
                            // for i in 0..pts.len() {
                            //     let (x1, y1) = transform.transform((pts[i][0], pts[i][1]));
                            //     canvas.rectangle(x1 as f32 - 1.0, y1 as f32 - 1.0, x1 as f32 + 1.0, y1 as f32 + 1.0, colors[(j + k) % colors.len()]);
                            // }
                        })
                    })
                    .collect::<Vec<_>>();

                target.lines(
                    lines,
                    DrawParameters {
                        polygon_mode: PolygonMode::Line,
                        line_width: Some(1.0),
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                );
            }
        } else if settings.debug && settings.draw_chunk_collision == 2 {
            if let Some(f) = &self.data.mesh_simplified {
                let colors = vec![
                    Color::rgb(32, 255, 32),
                    Color::rgb(255, 32, 32),
                    Color::rgb(32, 64, 255),
                    Color::rgb(255, 255, 32),
                    Color::rgb(32, 255, 255),
                    Color::rgb(255, 32, 255),
                ];

                let lines = f
                    .iter()
                    .enumerate()
                    .flat_map(|(j, f)| {
                        let c = colors[j % colors.len()];
                        f.iter().flat_map(move |pts| {
                            let mut v = vec![];
                            for i in 1..pts.len() {
                                let (x1, y1) = (pts[i - 1][0], pts[i - 1][1]);
                                let (x2, y2) = (pts[i][0], pts[i][1]);
                                v.push(((x1 as f32, y1 as f32), (x2 as f32, y2 as f32), (c)));
                            }
                            v
                        })
                    })
                    .collect::<Vec<_>>();

                target.lines(
                    lines,
                    DrawParameters {
                        polygon_mode: PolygonMode::Line,
                        line_width: Some(1.0),
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                );
            }
        } else if settings.debug && settings.draw_chunk_collision == 3 {
            if let Some(t) = &self.tris {
                let mut tris = vec![];

                for part in t {
                    for tri in part {
                        let (x1, y1) = tri.0;
                        let (x2, y2) = tri.1;
                        let (x3, y3) = tri.2;

                        let color = Color::rgba(32, 255, 255, 255);

                        tris.push((
                            (x1 as f32, y1 as f32),
                            (x2 as f32, y2 as f32),
                            (x3 as f32, y3 as f32),
                            color,
                        ));
                    }
                }

                target.triangles(
                    tris,
                    DrawParameters {
                        polygon_mode: PolygonMode::Line,
                        line_width: Some(1.0),
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                );
            }
        }
    }
}

impl ChunkGraphics {
    #[profiling::function]
    pub fn prep_render(
        &mut self,
        target: &mut RenderTarget,
        _settings: &Settings,
        _file_helper: &FileHelper,
    ) {
        if self.data.is_none() {
            let image = {
                glium::texture::RawImage2d {
                    data: Cow::Borrowed({
                        let color_sl = self.pixel_data.as_slice();
                        unsafe {
                            // Safety: Color is statically guaranteed to be equivalent to four u8s
                            core::slice::from_raw_parts(
                                color_sl.as_ptr().cast::<u8>(),
                                color_sl.len() * 4,
                            )
                        }
                    }),
                    width: CHUNK_SIZE.into(),
                    height: CHUNK_SIZE.into(),
                    format: glium::texture::ClientFormat::U8U8U8U8,
                }
            };
            let texture = Texture2d::with_format(
                &target.display,
                image,
                glium::texture::UncompressedFloatFormat::U8U8U8U8,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            let background_image = {
                glium::texture::RawImage2d {
                    data: Cow::Borrowed({
                        let color_sl = self.background_data.as_slice();
                        unsafe {
                            // Safety: Color is statically guaranteed to be equivalent to four u8s
                            core::slice::from_raw_parts(
                                color_sl.as_ptr().cast::<u8>(),
                                color_sl.len() * 4,
                            )
                        }
                    }),
                    width: CHUNK_SIZE.into(),
                    height: CHUNK_SIZE.into(),
                    format: glium::texture::ClientFormat::U8U8U8U8,
                }
            };
            let background_texture = Texture2d::with_format(
                &target.display,
                background_image,
                glium::texture::UncompressedFloatFormat::U8U8U8U8,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            let default_src = glium::texture::RawImage2d {
                data: Cow::Owned(vec![0.0; CHUNK_AREA * 4]),
                width: CHUNK_SIZE.into(),
                height: CHUNK_SIZE.into(),
                format: glium::texture::ClientFormat::F32F32F32F32,
            };

            let lighting_src = Texture2d::with_format(
                &target.display,
                default_src,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            let default_dst = glium::texture::RawImage2d {
                data: Cow::Owned(vec![
                    0.0;
                    ((CHUNK_SIZE / (LIGHT_SCALE as u16)) * (CHUNK_SIZE / (LIGHT_SCALE as u16)))
                        as usize
                        * 4
                ]),
                width: (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
                height: (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
                format: glium::texture::ClientFormat::F32F32F32F32,
            };

            let lighting_dst = Texture2d::with_format(
                &target.display,
                default_dst,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            let default_neighbors = glium::texture::RawImage2d {
                data: Cow::Owned(vec![
                    0.0;
                    ((CHUNK_SIZE / (LIGHT_SCALE as u16) + 2)
                        * (CHUNK_SIZE / (LIGHT_SCALE as u16) + 2))
                        as usize
                        * 4
                ]),
                width: (CHUNK_SIZE / (LIGHT_SCALE as u16) + 2).into(),
                height: (CHUNK_SIZE / (LIGHT_SCALE as u16) + 2).into(),
                format: glium::texture::ClientFormat::F32F32F32F32,
            };

            let lighting_neighbors = Texture2d::with_format(
                &target.display,
                default_neighbors,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            let constant_black = glium::texture::RawImage2d {
                data: Cow::Owned(vec![0.0, 0.0, 0.0, 1.0]),
                width: 1,
                height: 1,
                format: glium::texture::ClientFormat::F32F32F32F32,
            };

            let lighting_constant_black = Texture2d::with_format(
                &target.display,
                constant_black,
                glium::texture::UncompressedFloatFormat::F32F32F32F32,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();

            // lighting.write(rect, data)
            // let lighting = Texture2d::empty(&target.display, CHUNK_SIZE.into(), CHUNK_SIZE.into()).unwrap();

            self.data = Some(Arc::new(ChunkGraphicsData {
                display: target.display.clone(),
                texture,
                background_texture,
                lighting_src_buf: PixelBuffer::new_empty(&target.display, CHUNK_AREA),
                lighting_src,
                lighting_dst,
                lighting_neighbors,
                lighting_constant_black,
            }));
            self.dirty = true;
        }
    }
}

pub trait ClientChunkHandlerExt {
    fn sync_chunk(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        pixels: Vec<MaterialInstance>,
        colors: Vec<Color>,
    ) -> Result<(), String>;

    fn update_chunk_graphics(&mut self, shaders: &Shaders);
}

impl ClientChunkHandlerExt for ChunkHandler<ClientChunk> {
    fn sync_chunk(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        pixels: Vec<MaterialInstance>,
        colors: Vec<Color>,
    ) -> Result<(), String> {
        if pixels.len() != CHUNK_AREA {
            return Err(format!(
                "pixels Vec is the wrong size: {} (expected {})",
                pixels.len(),
                CHUNK_AREA
            ));
        }

        if colors.len() != CHUNK_AREA * 4 {
            return Err(format!(
                "colors Vec is the wrong size: {} (expected {})",
                colors.len(),
                CHUNK_AREA * 4
            ));
        }

        if let Some(chunk) = self.manager.chunk_at_mut((chunk_x, chunk_y)) {
            chunk.data.data.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
        } else {
            let mut chunk: ClientChunk = Chunk::new_empty(chunk_x, chunk_y);
            chunk.data.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
            self.manager.insert((chunk_x, chunk_y), chunk);
        }

        Ok(())
    }

    #[profiling::function]
    fn update_chunk_graphics(&mut self, shaders: &Shaders) {
        for ch in self.manager.chunks_iter_mut() {
            ch.graphics.was_dirty = ch.graphics.dirty;
            ch.graphics.was_lighting_dirty = ch.graphics.lighting_dirty;
        }

        self.manager
            .each_chunk_mut_with_surrounding_cardinal(|ch, others| {
                ch.data.update_graphics(Some(others), shaders).unwrap();
                ch.graphics.prev_dist_to_nearest_dirty_light =
                    ch.graphics.dist_to_nearest_dirty_light;
            });

        self.manager
            .each_chunk_mut_with_surrounding_cardinal(|ch, others| {
                let d = others
                    .iter()
                    .filter_map(|ch| ch.map(|ch| ch.graphics.prev_dist_to_nearest_dirty_light))
                    .flatten()
                    .min();
                ch.graphics.dist_to_nearest_dirty_light = None;
                if let Some(d) = d {
                    if d < 2 {
                        ch.graphics.dist_to_nearest_dirty_light = Some(d + 1);
                    }
                }
            });
    }
}
