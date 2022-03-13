use std::sync::Arc;

use fs_common::game::common::{world::{material::Color, particle::Particle}, Rect};
use glium::{Frame, Surface, SwapBuffersError, Display, DrawParameters, IndexBuffer, PolygonMode, index::NoIndices, uniform, Program, Texture2d, texture::SrgbTexture2d, implement_vertex};
use glium_glyph::{GlyphBrush, glyph_brush::Section};
use glutin::window::Window;

use super::{TransformStack, vertex::{Vertex2, Vertex2C, Vertex2T}, shaders::Shaders};

pub struct RenderTarget<'a, 'b> {
    pub frame: Frame,
    pub display: Display,
    pub transform: TransformStack,
    pub shaders: &'a Shaders,
    glyph_brush: &'a mut GlyphBrush<'b, 'b>,
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
        let shape = vec![(x1, y1).into(), (x2, y1).into(), (x2, y2).into(), (x1, y2).into()];
        shape
    }
}

impl Vertices for Rect<f32> {
    fn vertices(&self) -> Vec<Vertex2> {
        let x1 = self.left();
        let y1 = self.bottom();
        let x2 = self.right();
        let y2 = self.top();
        let shape = vec![(x1, y1).into(), (x2, y1).into(), (x2, y2).into(), (x1, y2).into()];
        shape
    }
}

impl<'a, 'b> RenderTarget<'a, 'b> {
    #[must_use]
    pub fn new(display: &mut Display, shaders: &'a Shaders, glyph_brush: &'a mut glium_glyph::GlyphBrush<'b, 'b>) -> Self {
        profiling::scope!("RenderTarget::new");
        
        Self {
            frame: display.draw(),
            display: display.clone(),
            transform: TransformStack::new(),
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
        self.frame.clear_color_srgb(color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32());
    }

    #[profiling::function]
    pub fn finish(self) -> Result<(), SwapBuffersError> {
        self.frame.finish()
    }

    pub fn triangle(&mut self, p1: impl Into<Vertex2>, p2: impl Into<Vertex2>, p3: impl Into<Vertex2>, color: Color, param: DrawParameters) {
        
        let p1 = p1.into();
        let p2 = p2.into();
        let p3 = p3.into();
        let shape = vec![p1, p2, p3];

        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);

        self.frame.draw(&vertex_buffer, &indices, &self.shaders.basic_shader, &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
    }

    pub fn rectangle(&mut self, rect: impl Into<Rect<f32>>, color: Color, param: DrawParameters) {
        let rect = rect.into();
        let shape = rect.vertices();

        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.basic_shader, 
                &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &[0_u8, 1, 2, 2, 3, 0]).unwrap();

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.basic_shader, 
                &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        }
    }

    pub fn rectangles(&mut self, rects: &[Rect<f32>], color: Color, param: DrawParameters) {
        let shape = rects.iter().flat_map(|rect| rect.vertices()).collect::<Vec<_>>();

        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.basic_shader, 
                &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let data = shape.iter().enumerate().flat_map(|(i, _)| {
                let base = (i * 4) as u16;
                [base, base + 1, base + 2, base + 2, base + 3, base]
            }).collect::<Vec<_>>();
            let indices = IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &data).unwrap();

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.basic_shader, 
                &uniform! { matrix: view, col: [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()] }, &param).unwrap();
        }
    }

    pub fn rectangles_colored(&mut self, rects: &[(Rect<f32>, Color)], param: DrawParameters) {
        let shape = rects.iter().copied().flat_map(|(rect, color)| rect.vertices().into_iter().map(move |v| Vertex2C::from((v, color)))).collect::<Vec<_>>();

        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        if param.polygon_mode == PolygonMode::Line {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let indices = NoIndices(glium::index::PrimitiveType::LineLoop);

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.shader_vertex_colors, 
                &uniform! { matrix: view }, &param).unwrap();
        } else {
            let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
            let data = shape.iter().enumerate().flat_map(|(i, _)| {
                let base = (i * 4) as u16;
                [base, base + 1, base + 2, base + 2, base + 3, base]
            }).collect::<Vec<_>>();
            let indices = IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &data).unwrap();

            self.frame.draw(&vertex_buffer, &indices, &self.shaders.shader_vertex_colors, 
                &uniform! { matrix: view }, &param).unwrap();
        }
    }

    pub fn queue_text(&mut self, section: Section) {
        self.glyph_brush.queue(section);
    }

    pub fn draw_queued_text(&mut self) {
        self.glyph_brush.draw_queued(&self.display, &mut self.frame);
    }

    pub fn draw_texture(&mut self, rect: impl Into<Rect<f32>>, texture: &SrgbTexture2d, param: DrawParameters) {
        let rect = rect.into();
        let shape = rect.vertices().into_iter().zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]).map(|v| Vertex2T::from(v)).collect::<Vec<_>>();

        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &[0_u8, 1, 2, 2, 3, 0]).unwrap();

        self.frame.draw(&vertex_buffer, &indices, &self.shaders.texture, 
            &uniform! { matrix: view, tex: texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest) }, &param).unwrap();
    }

    pub fn draw_particles(&mut self, parts: &[Particle]) {
        let model_view = *self.transform.stack.last().unwrap();
        let view: [[f32; 4]; 4] = model_view.into();

        let per_instance = {
            profiling::scope!("a");
            #[derive(Copy, Clone)]
            struct Attr {
                p_pos: (f32, f32),
                color: [f32; 4],
            }
    
            implement_vertex!(Attr, p_pos, color);
    
            let data = parts.iter().map(|p| {
                Attr {
                    p_pos: (p.pos.x as f32, p.pos.y as f32),
                    color: p.material.color.into(),
                }
            }).collect::<Vec<_>>();
    
            glium::vertex::VertexBuffer::immutable(&self.display, &data).unwrap()
        };

        let shape = Rect::<f32>::new(-0.5, -0.5, 0.5, 0.5).vertices();
        let vertex_buffer = glium::VertexBuffer::immutable(&self.display, &shape).unwrap();
        let indices = IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &[0_u8, 1, 2, 2, 3, 0]).unwrap();

        self.frame.draw((&vertex_buffer, per_instance.per_instance().unwrap()),
                    &indices, &self.shaders.particle, &uniform! { matrix: view },
                    &DrawParameters::default()).unwrap();
    }
}