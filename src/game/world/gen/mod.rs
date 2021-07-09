mod test;

pub use test::*;

use super::Chunk;

pub trait WorldGenerator {
    fn generate(&self, chunk: &mut Chunk);
}