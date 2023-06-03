use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::color::Color;

#[derive(Clone, Serialize, Deserialize)]
pub struct PostTickChunk {
    #[serde(with = "BigArray")]
    pub colors: [Color; 10000],
}
