use fs_common::game::common::{
    world::{material::color::Color, particle::Particle, CHUNK_SIZE},
    Rect,
};
use glium::{
    implement_vertex, index::NoIndices, texture::Texture2dArray, uniform, Blend, Display,
    DrawParameters, Frame, IndexBuffer, PolygonMode, Surface, SwapBuffersError, Texture2d,
};
use glium_glyph::{
    glyph_brush::{ab_glyph::FontVec, Section},
    GlyphBrush,
};

use super::{
    shaders::Shaders,
    vertex::{Vertex2, Vertex2C, Vertex2T, Vertex2TA},
    TransformStack,
};

pub struct RenderTarget<'a, 'b> {
    pub frame: Frame,
    pub display: Display,
    pub transform: TransformStack,
    pub base_transform: TransformStack,
    pub shaders: &'a Shaders,
    glyph_brush: &'a mut GlyphBrush<'b, FontVec>,
}

pub trait Vertices {
    fn vertices(&self) -> Vec<Vertex2>;
}

impl Vertices for Rect<i32> {
    fn vertices(&self) -> Vec<Vertex2> {
        let x1 = self.left() as f32;
        let y1 = self.bottom() as f32;
        let x2 = self.right() as f32;
        let y2 = self.top() as f32;
        let shape = vec![
            (x1, y1).into(),
            (x2, y1).into(),
            (x2, y2).into(),
            (x1, y2).into(),
        ];
        shape
    }
}

impl Vertices for Rect<f32> {
    fn vertices(&self) -> Vec<Vertex2> {
        let x1 = self.left();
        let y1 = self.bottom();
        let x2 = self.right();
        let y2 = self.top();
        let shape = vec![
            (x1, y1).into(),
            (x2, y1).into(),
            (x2, y2).into(),
            (x1, y2).into(),
        ];
        shape
    }
}

impl<'a, 'b> RenderTarget<'a, 'b> {
    #[must_use]
    pub fn new(
        display: &mut Display,
        shaders: &'a Shaders,
        glyph_brush: &'a mut glium_glyph::GlyphBrush<'b, FontVec>,
    ) -> Self {
        profiling::scope!("RenderTarget::new");

        Self {
            frame: display.draw(),
            display: display.clone(),
            transform: TransformStack::new(),
            base_transform: TransformStack::new(),
            shaders,
            glyph_brush,
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.display.gl_window().window().inner_size().width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.display.gl_window().window().inner_size().height
    }

    #[profiling::function]
    pub fn clear(&mut self, color: impl Into<Color>) {
        let color = color.into();
        self.frame
            .clear_color_srgb(color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32());
    }

    #[profiling::function]
    pub fn finish(self) -> Result<(), SwapBuffersError> {
        self.frame.finish()
    }

    pub fn line(
        &mut self,
        p1: impl Into<Vertex2>,
        p2: impl Into<Vertex2>,
        color: Color,
        param: DrawParameters,
    ) {
        let p1 = p1.into();
        let p2 = p2.into();
        let shape = vec![p1, p2];

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

        self.frame.draw(&vertex_buffer, indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
    }

    pub fn lines(
        &mut self,
        lines: Vec<(impl Into<Vertex2>, impl Into<Vertex2>, Color)>,
        param: DrawParameters,
    ) {
        let shape = lines
            .into_iter()
            .flat_map(|l| {
                let a = l.0.into();
                let b = l.1.into();
                [
                    Vertex2C { position: a.position, color: l.2.into() },
                    Vertex2C { position: b.position, color: l.2.into() },
                ]
            })
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::LinesList);

        self.frame
            .draw(
                &vertex_buffer,
                indices,
                &self.shaders.vertex_colors,
                &uniform! { matrix: view },
                &param,
            )
            .unwrap();
    }

    pub fn line_strip(&mut self, points: Vec<(impl Into<Vertex2>, Color)>, param: DrawParameters) {
        let shape = points
            .into_iter()
            .map(|(p, c)| {
                let p = p.into();
                Vertex2C { position: p.position, color: c.into() }
            })
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::LineStrip);

        self.frame
            .draw(
                &vertex_buffer,
                indices,
                &self.shaders.vertex_colors,
                &uniform! { matrix: view },
                &param,
            )
            .unwrap();
    }

    pub fn triangle(
        &mut self,
        p1: impl Into<Vertex2>,
        p2: impl Into<Vertex2>,
        p3: impl Into<Vertex2>,
        color: Color,
        param: DrawParameters,
    ) {
        let p1 = p1.into();
        let p2 = p2.into();
        let p3 = p3.into();
        let shape = vec![p1, p2, p3];

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);

        self.frame.draw(&vertex_buffer, indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
    }

    pub fn triangles(
        &mut self,
        tris: Vec<(
            impl Into<Vertex2>,
            impl Into<Vertex2>,
            impl Into<Vertex2>,
            Color,
        )>,
        param: DrawParameters,
    ) {
        let shape = tris
            .into_iter()
            .flat_map(|(a, b, c, color)| {
                let a = a.into();
                let b = b.into();
                let c = c.into();
                [
                    Vertex2C { position: a.position, color: color.into() },
                    Vertex2C { position: b.position, color: color.into() },
                    Vertex2C { position: c.position, color: color.into() },
                ]
            })
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        self.frame
            .draw(
                &vertex_buffer,
                indices,
                &self.shaders.vertex_colors,
                &uniform! { matrix: view },
                &param,
            )
            .unwrap();
    }

    pub fn rectangle(&mut self, rect: impl Into<Rect<f32>>, color: Color, param: DrawParameters) {
        let rect = rect.into();
        let shape = rect.vertices();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame.draw(&vertex_buffer, indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = IndexBuffer::new(
                &self.display,
                glium::index::PrimitiveType::TrianglesList,
                &[0_u8, 1, 2, 2, 3, 0],
            )
            .unwrap();

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        }
    }

    pub fn rectangles(&mut self, rects: &[Rect<f32>], color: Color, param: DrawParameters) {
        let shape = rects
            .iter()
            .flat_map(|rect| rect.vertices())
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame.draw(&vertex_buffer, indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let data = shape
                .iter()
                .enumerate()
                .flat_map(|(i, _)| {
                    let base = (i * 4) as u16;
                    [base, base + 1, base + 2, base + 2, base + 3, base]
                })
                .collect::<Vec<_>>();
            let indices = IndexBuffer::new(
                &self.display,
                glium::index::PrimitiveType::TrianglesList,
                &data,
            )
            .unwrap();

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.common, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        }
    }

    pub fn rectangles_colored(&mut self, rects: &[(Rect<f32>, Color)], param: DrawParameters) {
        let shape = rects
            .iter()
            .copied()
            .flat_map(|(rect, color)| {
                rect.vertices()
                    .into_iter()
                    .map(move |v| Vertex2C::from((v, color)))
            })
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame
                .draw(
                    &vertex_buffer,
                    indices,
                    &self.shaders.vertex_colors,
                    &uniform! { matrix: view },
                    &param,
                )
                .unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let data = shape
                .iter()
                .enumerate()
                .flat_map(|(i, _)| {
                    let base = (i * 4) as u16;
                    [base, base + 1, base + 2, base + 2, base + 3, base]
                })
                .collect::<Vec<_>>();
            let indices = IndexBuffer::new(
                &self.display,
                glium::index::PrimitiveType::TrianglesList,
                &data,
            )
            .unwrap();

            self.frame
                .draw(
                    &vertex_buffer,
                    &indices,
                    &self.shaders.vertex_colors,
                    &uniform! { matrix: view },
                    &param,
                )
                .unwrap();
        }
    }

    pub fn queue_text(&mut self, section: Section) {
        self.glyph_brush.queue(section);
    }

    pub fn draw_queued_text(&mut self) {
        self.glyph_brush.draw_queued(&self.display, &mut self.frame);
    }

    #[profiling::function]
    pub fn draw_texture(
        &mut self,
        rect: impl Into<Rect<f32>>,
        texture: &Texture2d,
        param: DrawParameters,
    ) {
        let rect = rect.into();
        let shape = rect
            .vertices()
            .into_iter()
            .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
            .map(Vertex2T::from)
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TriangleStrip,
            &[1_u16, 2, 0, 3],
        )
        .unwrap();

        {
            profiling::scope!("draw");
            self.frame.draw(&vertex_buffer, &indices, &self.shaders.texture, &uniform! { matrix: view, tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest) }, &param).unwrap();
        }
    }

    #[profiling::function]
    pub fn draw_texture_flipped(
        &mut self,
        rect: impl Into<Rect<f32>>,
        texture: &Texture2d,
        param: DrawParameters,
    ) {
        let rect = rect.into();
        let shape = rect
            .vertices()
            .into_iter()
            .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
            .map(Vertex2T::from)
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TriangleStrip,
            &[1_u16, 2, 0, 3],
        )
        .unwrap();

        {
            profiling::scope!("draw");
            self.frame.draw(&vertex_buffer, &indices, &self.shaders.texture, &uniform! { matrix: view, tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest) }, &param).unwrap();
        }
    }

    #[profiling::function]
    pub fn draw_textures(
        &mut self,
        rects: &[Rect<f32>],
        texture: &Texture2dArray,
        param: DrawParameters,
    ) {
        let shape = rects
            .iter()
            .flat_map(|rect| {
                rect.vertices()
                    .into_iter()
                    .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
                    .enumerate()
                    .map(|(layer, (v, t))| {
                        Vertex2TA::from((
                            (v.position[0], v.position[1]),
                            (t[0], t[1]),
                            layer as f32,
                        ))
                    })
            })
            .collect::<Vec<_>>();

        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TriangleStrip,
            &[1_u16, 2, 0, 3],
        )
        .unwrap();

        self.frame.draw(&vertex_buffer, &indices, &self.shaders.texture_array, &uniform! { matrix: view, tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest) }, &param).unwrap();
    }

    pub fn draw_particles(&mut self, parts: &[Particle], partial_ticks: f32) {
        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let per_instance = {
            profiling::scope!("a");
            #[derive(Copy, Clone)]
            struct Attr {
                p_pos: (f32, f32),
                color: [f32; 4],
            }

            implement_vertex!(Attr, p_pos, color);

            let data = parts
                .iter()
                .map(|p| Attr {
                    p_pos: (
                        p.pos.x as f32 + p.vel.x as f32 * partial_ticks,
                        p.pos.y as f32 + p.vel.y as f32 * partial_ticks,
                    ),
                    color: p.material.color.into(),
                })
                .collect::<Vec<_>>();

            glium::vertex::VertexBuffer::immutable(&self.display, &data).unwrap()
        };

        let shape = Rect::<f32>::new(-0.5, -0.5, 0.5, 0.5).vertices();
        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TrianglesList,
            &[0_u8, 1, 2, 2, 3, 0],
        )
        .unwrap();

        self.frame
            .draw(
                (&vertex_buffer, per_instance.per_instance().unwrap()),
                &indices,
                &self.shaders.particle,
                &uniform! { matrix: view },
                &DrawParameters::default(),
            )
            .unwrap();
    }

    pub fn draw_chunks(&mut self, chunks: &[(f32, f32)], texture_array: &Texture2dArray) {
        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let per_instance = {
            profiling::scope!("a");
            #[derive(Copy, Clone)]
            struct Attr {
                c_pos: (f32, f32),
                tex_layer: f32,
            }

            implement_vertex!(Attr, c_pos, tex_layer);

            let data = chunks
                .iter()
                .enumerate()
                .map(|(i, p)| Attr { c_pos: *p, tex_layer: i as f32 })
                .collect::<Vec<_>>();

            glium::vertex::VertexBuffer::immutable(&self.display, &data).unwrap()
        };

        let shape = Rect::<f32>::new(0.0, 0.0, CHUNK_SIZE as f32, CHUNK_SIZE as f32)
            .vertices()
            .into_iter()
            .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
            .map(Vertex2T::from)
            .collect::<Vec<_>>();
        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TriangleStrip,
            &[1_u16, 2, 0, 3],
        )
        .unwrap();

        {
            profiling::scope!("draw");
            self.frame
                .draw(
                    (&vertex_buffer, per_instance.per_instance().unwrap()),
                    &indices,
                    &self.shaders.texture_array,
                    &uniform! { matrix: view, tex: texture_array },
                    &DrawParameters::default(),
                )
                .unwrap();
        }
    }

    #[profiling::function]
    pub fn draw_chunks_2(&mut self, chunks: Vec<((f32, f32), &Texture2d)>) {
        let model_view =
            *self.base_transform.stack.last().unwrap() * *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let shape = Rect::<f32>::new(0.0, 0.0, CHUNK_SIZE as f32, CHUNK_SIZE as f32)
            .vertices()
            .into_iter()
            .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
            .map(Vertex2T::from)
            .collect::<Vec<_>>();
        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(
            &self.display,
            glium::index::PrimitiveType::TriangleStrip,
            &[1_u16, 2, 0, 3],
        )
        .unwrap();

        let params = DrawParameters {
            blend: Blend::alpha_blending(),
            ..DrawParameters::default()
        };

        for (p, texture) in chunks {
            profiling::scope!("draw chunk");
            self.frame.draw(&vertex_buffer, &indices, &self.shaders.chunk, &uniform! { matrix: view, c_pos: p, tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest) }, &params).unwrap();
        }
    }
}
