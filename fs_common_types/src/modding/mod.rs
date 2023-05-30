use serde::{Deserialize, Serialize};

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
