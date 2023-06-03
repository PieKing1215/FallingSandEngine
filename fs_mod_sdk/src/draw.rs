use fs_common_types::{color::Color, rect::Rect};

wasm_plugin_guest::import_functions! {
    fn RenderTarget_width() -> u32;
    fn RenderTarget_height() -> u32;
    fn RenderTarget_rectangle(v: (Rect<f32>, Color));
    fn RenderTarget_rectangle_filled(v: (Rect<f32>, Color));
}

pub struct RenderTarget {
    _private: (),
}

impl RenderTarget {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    pub fn width(&self) -> u32 {
        RenderTarget_width()
    }

    pub fn height(&self) -> u32 {
        RenderTarget_height()
    }

    pub fn rectangle(&mut self, rect: Rect<f32>, color: Color) {
        RenderTarget_rectangle((rect, color));
    }

    pub fn rectangle_filled(&mut self, rect: Rect<f32>, color: Color) {
        RenderTarget_rectangle_filled((rect, color));
    }
}
