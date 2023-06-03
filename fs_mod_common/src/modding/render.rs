use crate::{color::Color, rect::Rect};

pub trait RenderTarget {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn rectangle(&mut self, rect: Rect<f32>, color: Color);
    fn rectangle_filled(&mut self, rect: Rect<f32>, color: Color);
}
