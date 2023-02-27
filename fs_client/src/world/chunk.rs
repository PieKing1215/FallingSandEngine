use std::{borrow::Cow, collections::HashMap, convert::TryInto, hash::BuildHasherDefault};

use fs_common::game::common::{
    world::{
        chunk_index,
        material::{color::Color, MaterialInstance},
        mesh, Chunk, ChunkHandler, ChunkState, PassThroughHasherU32, RigidBodyState, CHUNK_SIZE,
        LIGHT_SCALE,
    },
    FileHelper, Rect, Settings,
};
use glium::{
    program::ComputeShader, texture::Texture2d, uniform, uniforms::ImageUnit, Blend, Display,
    DrawParameters, PolygonMode, Program,
};

use crate::render::{drawing::RenderTarget, shaders::ShaderFileHelper};

pub struct ClientChunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub light: Option<Box<[f32; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub graphics: Box<ChunkGraphics>,
    pub dirty_rect: Option<Rect<i32>>,
    pub rigidbody: Option<RigidBodyState>,
    pub mesh: Option<Vec<Vec<Vec<Vec<f64>>>>>,
    pub mesh_simplified: Option<Vec<Vec<Vec<Vec<f64>>>>>,
    pub tris: Option<Vec<Vec<mesh::Tri>>>,
}

impl Chunk for ClientChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            light: None,
            graphics: Box::new(ChunkGraphics {
                data: None,
                pixel_data: Box::new([0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)]),
                lighting_data: Box::new([0.0; CHUNK_SIZE as usize * CHUNK_SIZE as usize]),
                dirty: true,
                was_dirty: true,
                lighting_dirty: true,
            }),
            dirty_rect: None,
            rigidbody: None,
            mesh: None,
            mesh_simplified: None,
            tris: None,
        }
    }

    fn get_chunk_x(&self) -> i32 {
        self.chunk_x
    }

    fn get_chunk_y(&self) -> i32 {
        self.chunk_y
    }

    fn get_state(&self) -> ChunkState {
        self.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.state = state;
    }

    fn get_dirty_rect(&self) -> Option<Rect<i32>> {
        self.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>) {
        self.dirty_rect = rect;
    }

    fn refresh(&mut self) {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                self.graphics
                    .set(
                        x,
                        y,
                        self.pixels.as_ref().unwrap()[(x + y * CHUNK_SIZE) as usize].color,
                    )
                    .unwrap();
                self.graphics
                    .set_light(
                        x,
                        y,
                        self.light.as_ref().unwrap()[(x + y * CHUNK_SIZE) as usize],
                    )
                    .unwrap();
            }
        }
        self.update_graphics(None).unwrap();
    }

    // #[profiling::function]
    fn update_graphics(
        &mut self,
        other_loaded_chunks: Option<&HashMap<u32, Self, BuildHasherDefault<PassThroughHasherU32>>>,
    ) -> Result<(), String> {
        self.graphics.was_dirty = self.graphics.dirty;

        self.graphics.update_texture();
        self.graphics.update_lighting(other_loaded_chunks.map(|ch| {
            [
                ch.get(&chunk_index(self.chunk_x - 1, self.chunk_y - 1)),
                ch.get(&chunk_index(self.chunk_x, self.chunk_y - 1)),
                ch.get(&chunk_index(self.chunk_x + 1, self.chunk_y - 1)),
                ch.get(&chunk_index(self.chunk_x - 1, self.chunk_y)),
                ch.get(&chunk_index(self.chunk_x + 1, self.chunk_y)),
                ch.get(&chunk_index(self.chunk_x - 1, self.chunk_y + 1)),
                ch.get(&chunk_index(self.chunk_x, self.chunk_y + 1)),
                ch.get(&chunk_index(self.chunk_x + 1, self.chunk_y + 1)),
            ]
        }));

        Ok(())
    }

    // #[profiling::function] // huge performance impact
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                self.graphics.set(x, y, mat.color)?;
                self.graphics.set_light(x, y, mat.light)?;
                *unsafe { px.get_unchecked_mut(i) } = mat;

                self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn set_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        self.graphics.set(x, y, mat.color).unwrap();
        self.graphics.set_light(x, y, mat.light).unwrap();
        *unsafe { self.pixels.as_mut().unwrap().get_unchecked_mut(i) } = mat;

        self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));
    }

    // #[profiling::function] // huge performance impact
    fn get(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { px.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn get_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.pixels.as_ref().unwrap().get_unchecked(i) }
    }

    fn replace<F>(&mut self, x: u16, y: u16, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                let px = unsafe { px.get_unchecked_mut(i) };
                if let Some(mat) = (cb)(px) {
                    self.graphics.set(x, y, mat.color)?;
                    *px = mat;

                    self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                    return Ok(true);
                }

                return Ok(false);
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    // #[profiling::function] // huge performance impact
    fn set_light(&mut self, x: u16, y: u16, light: f32) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(li) = &mut self.light {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                self.graphics.set_light(x, y, light)?;
                *unsafe { li.get_unchecked_mut(i) } = light;

                // self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn set_light_unchecked(&mut self, x: u16, y: u16, light: f32) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        self.graphics.set_light(x, y, light).unwrap();
        *unsafe { self.light.as_mut().unwrap().get_unchecked_mut(i) } = light;

        // self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));
    }

    // #[profiling::function] // huge performance impact
    fn get_light(&self, x: u16, y: u16) -> Result<&f32, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(li) = &self.light {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { li.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn get_light_unchecked(&self, x: u16, y: u16) -> &f32 {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.light.as_ref().unwrap().get_unchecked(i) }
    }

    fn set_color(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            return self.graphics.set(x, y, color);
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    fn get_color(&self, x: u16, y: u16) -> Result<Color, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            return self.graphics.get(x, y);
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    fn apply_diff(&mut self, diff: &[(u16, u16, MaterialInstance)]) {
        for (x, y, mat) in diff {
            self.set(*x, *y, mat.clone()).unwrap(); // TODO: handle this Err
        }
    }

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>) {
        self.pixels = Some(pixels);
        self.light = Some(Box::new([0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize]));
        self.refresh();
    }

    fn get_pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &mut self.pixels
    }

    fn get_pixels(&self) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &self.pixels
    }

    fn set_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    ) {
        self.graphics.replace(colors);
    }

    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.graphics.pixel_data
    }

    fn get_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.graphics.pixel_data
    }

    fn mark_dirty(&mut self) {
        self.graphics.dirty = true;
        self.graphics.lighting_dirty = true;
    }

    fn generate_mesh(&mut self) -> Result<(), String> {
        if self.pixels.is_none() {
            return Err("generate_mesh failed: self.pixels is None".to_owned());
        }

        let vs: Vec<f64> = mesh::pixels_to_valuemap(self.pixels.as_ref().unwrap().as_ref());

        let generated =
            mesh::generate_mesh_with_simplified(&vs, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));

        if let Ok(r) = generated {
            self.mesh = Some(r.0);
            self.mesh_simplified = Some(r.1);
        } else {
            self.mesh = None;
            self.mesh_simplified = None;
        }

        self.tris = self.mesh_simplified.as_ref().map(mesh::triangulate);

        Ok(())
    }

    // fn get_tris(&self) -> &Option<Vec<Vec<((f64, f64), (f64, f64), (f64, f64))>>> {
    //     &self.tris
    // }

    fn get_mesh_loops(&self) -> &Option<Vec<Vec<Vec<Vec<f64>>>>> {
        &self.mesh_simplified
    }

    fn get_rigidbody(&self) -> &Option<RigidBodyState> {
        &self.rigidbody
    }

    fn get_rigidbody_mut(&mut self) -> &mut Option<RigidBodyState> {
        &mut self.rigidbody
    }

    fn set_rigidbody(&mut self, body: Option<RigidBodyState>) {
        self.rigidbody = body;
    }

    fn get_lights_mut(&mut self) -> &mut [f32; CHUNK_SIZE as usize * CHUNK_SIZE as usize] {
        &mut self.graphics.lighting_data
    }

    fn get_lights(&self) -> &[f32; CHUNK_SIZE as usize * CHUNK_SIZE as usize] {
        &self.graphics.lighting_data
    }
}

pub struct ChunkGraphicsData {
    pub display: Display,
    pub texture: Texture2d,
    pub lighting_src: Texture2d,
    pub lighting_dst: Texture2d,
    pub lighting_neighbors: Texture2d,
    pub lighting_constant_black: Texture2d,
    pub lighting_shader: Program,
    pub lighting_compute_propagate: ComputeShader,
    pub lighting_compute_prep: ComputeShader,
}

pub struct ChunkGraphics {
    pub data: Option<ChunkGraphicsData>,
    pub pixel_data: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    pub lighting_data: Box<[f32; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,
    pub dirty: bool,
    pub was_dirty: bool,
    pub lighting_dirty: bool,
}

unsafe impl Send for ChunkGraphics {}
unsafe impl Sync for ChunkGraphics {}

impl ChunkGraphics {
    // #[profiling::function] // huge performance impact
    pub fn set(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            // self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            let i = (x + y * CHUNK_SIZE) as usize;
            self.pixel_data[i * 4] = color.r;
            self.pixel_data[i * 4 + 1] = color.g;
            self.pixel_data[i * 4 + 2] = color.b;
            self.pixel_data[i * 4 + 3] = color.a;
            self.dirty = true;
            self.lighting_dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    // #[profiling::function] // huge performance impact
    pub fn set_light(&mut self, x: u16, y: u16, color: f32) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            // self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            let i = (x + y * CHUNK_SIZE) as usize;
            self.lighting_data[i] = color;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    // #[profiling::function] // huge performance impact
    pub fn get(&self, x: u16, y: u16) -> Result<Color, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            // self.surface.fill_rect(Rect::new(x as i32, y as i32, 1, 1), color)?;
            let i = (x + y * CHUNK_SIZE) as usize;

            return Ok(Color::rgba(
                self.pixel_data[i * 4],
                self.pixel_data[i * 4 + 1],
                self.pixel_data[i * 4 + 2],
                self.pixel_data[i * 4 + 3],
            ));
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    #[profiling::function]
    pub fn update_texture(&mut self) {
        if self.dirty {
            if let Some(data) = &mut self.data {
                let image = glium::texture::RawImage2d::from_raw_rgba(
                    self.pixel_data.to_vec(),
                    (CHUNK_SIZE.into(), CHUNK_SIZE.into()),
                );

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
            self.dirty = false;
        }
    }

    #[profiling::function]
    pub fn update_lighting(&mut self, neighbors: Option<[Option<&ClientChunk>; 8]>) {
        if self.lighting_dirty
            || (neighbors.map_or(false, |n| {
                n.iter()
                    .any(|c| c.map_or(false, |c| c.graphics.dirty || c.graphics.was_dirty))
            }))
        {
            if let Some(data) = &mut self.data {
                profiling::scope!("lighting update");
                // self.lighting_data = Box::new([0.0; CHUNK_SIZE as usize * CHUNK_SIZE as usize]);

                // // self.lighting_data[(30 / LIGHT_SCALE as usize) + (30 / LIGHT_SCALE as usize) * (CHUNK_SIZE / (LIGHT_SCALE as u16)) as usize] = 1.0;
                // // self.lighting_data[(60 / LIGHT_SCALE as usize) + (50 / LIGHT_SCALE as usize) * (CHUNK_SIZE / (LIGHT_SCALE as u16)) as usize] = 1.0;
                // let start = std::time::SystemTime::now();
                // let time = start.duration_since(std::time::UNIX_EPOCH).unwrap();
                // let x = 10 + ((((time.as_millis() % 2000) as f32 / 2000.0 * 2.0 * std::f32::consts::PI).sin() + 1.0) * 40.0) as usize;
                // let y = 10 + ((((time.as_millis() % 1300) as f32 / 1300.0 * 2.0 * std::f32::consts::PI).cos() + 1.0) * 40.0) as usize;
                // // log::debug!("{x} {} {}", time.as_millis(), ((time.as_millis() % 1000) as f32 / 1000.0).sin());
                // if neighbors.map_or(false, |n| n[1].map_or(false, |c| c.chunk_x % 3 == 0 && c.chunk_y % 2 == 0)) {
                //     self.lighting_data[x + y * CHUNK_SIZE as usize] = 1.0;
                // }

                let image = glium::texture::RawImage2d {
                    data: Cow::Owned(self.lighting_data.to_vec()),
                    width: CHUNK_SIZE.into(),
                    height: CHUNK_SIZE.into(),
                    format: glium::texture::ClientFormat::F32,
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
                        image,
                    );
                }

                fn r32f_read(tex: &Texture2d) -> ImageUnit<Texture2d> {
                    tex.image_unit(glium::uniforms::ImageUnitFormat::R32F)
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
                    .image_unit(glium::uniforms::ImageUnitFormat::R32F)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::Write);
                let t_work = data
                    .lighting_neighbors
                    .image_unit(glium::uniforms::ImageUnitFormat::R32F)
                    .unwrap()
                    .set_access(glium::uniforms::ImageUnitAccess::ReadWrite);

                let t_light_n = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[1].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_w = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[3].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_e = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[4].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
                        })
                        .unwrap_or(&data.lighting_constant_black),
                );
                let t_light_s = r32f_read(
                    neighbors
                        .and_then(|ch| {
                            ch[6].and_then(|c| c.graphics.data.as_ref().map(|d| &d.lighting_dst))
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
                    data.lighting_compute_prep.execute(uni, 1, 1, 1);
                }

                let t_work = data
                    .lighting_neighbors
                    .image_unit(glium::uniforms::ImageUnitFormat::R32F)
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
                    data.lighting_compute_propagate.execute(uni, 1, 1, 1);
                }
            }
            self.lighting_dirty = false;
        }
    }

    #[profiling::function]
    #[allow(clippy::cast_lossless)]
    pub fn replace(
        &mut self,
        colors: Box<[u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize]>,
    ) {
        // let sf = Surface::from_data(&mut colors, CHUNK_SIZE as u32, CHUNK_SIZE as u32, self.surface.pitch(), self.surface.pixel_format_enum()).unwrap();
        // sf.blit(None, &mut self.surface, None).unwrap();
        self.pixel_data = colors;
        self.dirty = true;
    }
}

impl ClientChunk {
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
            if let Some(f) = &self.mesh_simplified {
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
        file_helper: &FileHelper,
    ) {
        if self.data.is_none() {
            let image = glium::texture::RawImage2d::from_raw_rgba(
                self.pixel_data.to_vec(),
                (CHUNK_SIZE.into(), CHUNK_SIZE.into()),
            );
            let texture = Texture2d::new(&target.display, image).unwrap();

            // let lighting_src_img = glium::texture::RawImage2d {
            //     data: Cow::Owned(self.lighting_data.to_vec()),
            //     width: (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
            //     height: (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
            //     format: glium::texture::ClientFormat::F32,
            // };

            let lighting_src = Texture2d::empty_with_format(
                &target.display,
                glium::texture::UncompressedFloatFormat::F32,
                glium::texture::MipmapsOption::NoMipmap,
                CHUNK_SIZE.into(),
                CHUNK_SIZE.into(),
            )
            .unwrap();

            let lighting_dst = Texture2d::empty_with_format(
                &target.display,
                glium::texture::UncompressedFloatFormat::F32,
                glium::texture::MipmapsOption::NoMipmap,
                (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
                (CHUNK_SIZE / (LIGHT_SCALE as u16)).into(),
            )
            .unwrap();

            let lighting_neighbors = Texture2d::empty_with_format(
                &target.display,
                glium::texture::UncompressedFloatFormat::F32,
                glium::texture::MipmapsOption::NoMipmap,
                (CHUNK_SIZE / (LIGHT_SCALE as u16) + 2).into(),
                (CHUNK_SIZE / (LIGHT_SCALE as u16) + 2).into(),
            )
            .unwrap();

            let lighting_constant_black = Texture2d::empty_with_format(
                &target.display,
                glium::texture::UncompressedFloatFormat::F32,
                glium::texture::MipmapsOption::NoMipmap,
                1,
                1,
            )
            .unwrap();

            // lighting.write(rect, data)
            // let lighting = Texture2d::empty(&target.display, CHUNK_SIZE.into(), CHUNK_SIZE.into()).unwrap();

            let helper = ShaderFileHelper { file_helper, display: &target.display };

            let lighting_shader = helper
                .load_from_files(
                    140,
                    "data/shaders/chunk_lighting.vert",
                    "data/shaders/chunk_lighting.frag",
                )
                .unwrap();

            let lighting_compute_propagate = helper
                .load_compute_from_files("data/shaders/lighting_propagate.comp")
                .unwrap();

            let lighting_compute_prep = helper
                .load_compute_from_files("data/shaders/lighting_prep.comp")
                .unwrap();

            self.data = Some(ChunkGraphicsData {
                display: target.display.clone(),
                texture,
                lighting_src,
                lighting_dst,
                lighting_neighbors,
                lighting_constant_black,
                lighting_shader,
                lighting_compute_propagate,
                lighting_compute_prep,
            });
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
        colors: Vec<u8>,
    ) -> Result<(), String>;
}

impl ClientChunkHandlerExt for ChunkHandler<ClientChunk> {
    fn sync_chunk(
        &mut self,
        chunk_x: i32,
        chunk_y: i32,
        pixels: Vec<MaterialInstance>,
        colors: Vec<u8>,
    ) -> Result<(), String> {
        if pixels.len() != (CHUNK_SIZE * CHUNK_SIZE) as usize {
            return Err(format!(
                "pixels Vec is the wrong size: {} (expected {})",
                pixels.len(),
                CHUNK_SIZE * CHUNK_SIZE
            ));
        }

        if colors.len() != CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4 {
            return Err(format!(
                "colors Vec is the wrong size: {} (expected {})",
                colors.len(),
                CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4
            ));
        }

        if let Some(chunk) = self.loaded_chunks.get_mut(&chunk_index(chunk_x, chunk_y)) {
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
        } else {
            let mut chunk: ClientChunk = Chunk::new_empty(chunk_x, chunk_y);
            chunk.pixels = Some(pixels.try_into().unwrap());
            chunk.graphics.pixel_data = colors.try_into().unwrap();
            chunk.mark_dirty();
            chunk.set_state(ChunkState::Cached);
            self.loaded_chunks
                .insert(chunk_index(chunk_x, chunk_y), chunk);
        }

        Ok(())
    }
}
