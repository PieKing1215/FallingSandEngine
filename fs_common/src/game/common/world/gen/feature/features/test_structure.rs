use rand::Rng;

use crate::game::common::world::{
    gen::{feature::ConfiguredFeature, structure::StructureNode},
    Position, CHUNK_SIZE,
};

#[derive(Debug)]
pub struct TestStructure;

impl ConfiguredFeature for TestStructure {
    fn try_place(
        &self,
        chunks: &mut crate::game::common::world::gen::populator::ChunkContext<1>,
        pos: (i32, i32),
        _seed: i32,
        rng: &mut dyn rand::RngCore,
        _registries: &crate::game::Registries,
        ecs: &mut specs::World,
    ) {
        let (cx, cy) = chunks.center_chunk();
        let x = i64::from(cx * i32::from(CHUNK_SIZE)) + i64::from(pos.0);
        let y = i64::from(cy * i32::from(CHUNK_SIZE)) + i64::from(pos.1);
        StructureNode::create_and_add(ecs, Position { x: x as _, y: y as _ }, 2, rng.gen());
    }
}
