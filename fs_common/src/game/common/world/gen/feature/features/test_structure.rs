use rand::Rng;
use specs::{Join, ReadStorage};

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

        let (position_storage, node_storage) =
            ecs.system_data::<(ReadStorage<Position>, ReadStorage<StructureNode>)>();

        let ok = (&position_storage, &node_storage)
            .join()
            .all(|(pos, node)| {
                node.parent.is_some() || {
                    let dx = pos.x - x as f64;
                    let dy = pos.y - y as f64;
                    let dist = dx * dx + dy * dy;
                    dist > 1000.0 * 1000.0
                }
            });

        drop(position_storage);
        drop(node_storage);

        if ok {
            StructureNode::create_and_add(ecs, Position { x: x as _, y: y as _ }, 3, rng.gen());
        }
    }
}
