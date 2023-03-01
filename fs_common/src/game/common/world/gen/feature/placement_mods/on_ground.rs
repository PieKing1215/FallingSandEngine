use crate::game::common::{
    world::{
        gen::{feature::PlacementModifier, populator::ChunkContext},
        material::PhysicsType,
        CHUNK_SIZE,
    },
    Registries,
};

#[derive(Debug)]
pub struct OnGround {
    pub max_distance: Option<u32>,
}

impl PlacementModifier for OnGround {
    fn process(
        &self,
        chunks: &mut ChunkContext<1>,
        mut pos: (i32, i32),
        _seed: i32,
        _rng: &mut dyn rand::RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        let mut dist = 0;
        while chunks.get(pos.0, pos.1).unwrap().physics != PhysicsType::Solid {
            dist += 1;

            if pos.1 + 1 >= i32::from(CHUNK_SIZE * 2)
                || (self.max_distance.is_some() && dist > self.max_distance.unwrap())
            {
                return vec![];
            }

            pos.1 += 1;
        }

        vec![pos]
    }
}
