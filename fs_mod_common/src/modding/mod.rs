pub mod render;

use serde::{Deserialize, Serialize};

use crate::color::Color;

use self::render::RenderTarget;

#[derive(Debug, Clone, derive_getters::Getters, Serialize, Deserialize)]
pub struct ModMeta {
    id: String,
    display_name: Option<String>,
}

impl ModMeta {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), display_name: None }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

#[allow(unused_variables)]
pub trait Mod {
    fn meta(&self) -> &ModMeta;
    fn post_world_render(&mut self, target: &mut dyn RenderTarget) {}
    fn post_chunk_simulate(&mut self, colors: &mut [Color; 10000]) {}
}
