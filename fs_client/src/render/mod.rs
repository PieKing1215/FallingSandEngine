mod renderer;
use fs_common::game::common::Settings;
pub use renderer::*;

mod imgui;

use sdl2::rect::Rect;
use sdl_gpu::GPUTarget;

pub type RenderCanvas = sdl_gpu::GPUTarget;

#[derive(Clone)]
pub struct TransformStack {
    stack: Vec<Transform>,
}

impl TransformStack {
    pub fn new() -> Self {
        TransformStack {
            stack: vec![Transform {
                translate_x: 0.0,
                translate_y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
            }],
        }
    }

    pub fn push(&mut self) {
        self.stack.push(self.stack.last().unwrap().clone());
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn translate<T: Into<f64>>(&mut self, x: T, y: T) {
        self.stack.last_mut().unwrap().translate_x += x.into();
        self.stack.last_mut().unwrap().translate_y += y.into();
    }

    pub fn scale<T: Into<f64>>(&mut self, x: T, y: T) {
        let prev_x = self.stack.last_mut().unwrap().scale_x;
        let prev_y = self.stack.last_mut().unwrap().scale_y;

        self.stack.last_mut().unwrap().scale_x *= x.into();
        self.stack.last_mut().unwrap().scale_y *= y.into();
        self.stack.last_mut().unwrap().translate_x /=
            self.stack.last_mut().unwrap().scale_x / prev_x;
        self.stack.last_mut().unwrap().translate_y /=
            self.stack.last_mut().unwrap().scale_y / prev_y;
    }

    #[inline(always)]
    pub fn transform<T: Into<f64>>(&self, point: (T, T)) -> (f64, f64) {
        let t = self.stack.last().unwrap();
        (
            (point.0.into() + t.translate_x) * t.scale_x,
            (point.1.into() + t.translate_y) * t.scale_y,
        )
    }

    #[inline(always)]
    pub fn transform_int<T: Into<f64>>(&self, point: (T, T)) -> (i32, i32) {
        let t = self.stack.last().unwrap();
        (
            ((point.0.into() + t.translate_x) * t.scale_x) as i32,
            ((point.1.into() + t.translate_y) * t.scale_y) as i32,
        )
    }

    pub fn transform_rect(&self, rect: Rect) -> Rect {
        let pos = self.transform_int((rect.x, rect.y));

        let t = self.stack.last().unwrap();
        Rect::new(
            pos.0,
            pos.1,
            (f64::from(rect.w) * t.scale_x).ceil() as u32,
            (f64::from(rect.h) * t.scale_y).ceil() as u32,
        )
    }

    #[allow(dead_code)]
    pub fn inv_transform<T: Into<f64>>(&self, point: (T, T)) -> (f64, f64) {
        let t = self.stack.last().unwrap();
        (
            point.0.into() / t.scale_x - t.translate_x,
            point.1.into() / t.scale_y - t.translate_y,
        )
    }

    #[allow(dead_code)]
    pub fn inv_transform_int<T: Into<f64>>(&self, point: (T, T)) -> (i32, i32) {
        let t = self.stack.last().unwrap();
        (
            (point.0.into() / t.scale_x - t.translate_x) as i32,
            (point.1.into() / t.scale_y - t.translate_y) as i32,
        )
    }

    #[allow(dead_code)]
    pub fn inv_transform_rect(&self, rect: Rect) -> Rect {
        let pos = self.inv_transform_int((rect.x, rect.y));

        let t = self.stack.last().unwrap();
        Rect::new(
            pos.0,
            pos.1,
            (f64::from(rect.w) / t.scale_x) as u32,
            (f64::from(rect.h) / t.scale_y) as u32,
        )
    }
}

impl Default for TransformStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct Transform {
    translate_x: f64,
    translate_y: f64,
    scale_x: f64,
    scale_y: f64,
}

pub trait Renderable {
    fn render(
        &self,
        target: &mut GPUTarget,
        transform: &mut TransformStack,
        sdl: &Sdl2Context,
        fonts: &Fonts,
        settings: &Settings,
    );
}
