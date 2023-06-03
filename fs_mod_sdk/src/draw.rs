use fs_mod_common::{color::Color, modding::render::RenderTarget, rect::Rect};

wasm_plugin_guest::import_functions! {
    fn RenderTarget_width() -> u32;
    fn RenderTarget_height() -> u32;
    fn RenderTarget_rectangle(v: (Rect<f32>, Color));
    fn RenderTarget_rectangle_filled(v: (Rect<f32>, Color));
}

pub(crate) struct DummyRT {
    _private: (),
}

impl DummyRT {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl RenderTarget for DummyRT {
    fn width(&self) -> u32 {
        RenderTarget_width()
    }

    fn height(&self) -> u32 {
        RenderTarget_height()
    }

    fn rectangle(&mut self, rect: Rect<f32>, color: Color) {
        RenderTarget_rectangle((rect, color));
    }

    fn rectangle_filled(&mut self, rect: Rect<f32>, color: Color) {
        RenderTarget_rectangle_filled((rect, color));
    }
}
