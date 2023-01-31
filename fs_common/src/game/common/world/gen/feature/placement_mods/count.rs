use std::{fmt::Debug, sync::Arc};

use rand::{distributions::uniform::SampleRange, Rng};

use crate::game::{
    common::world::gen::{feature::PlacementModifier, populator::ChunkContext},
    Registries,
};

pub type CountFn = dyn Fn(&mut dyn rand::RngCore) -> u16 + Send + Sync;

pub struct Count {
    func: Arc<CountFn>,
}

impl Count {
    pub fn new(func: Arc<CountFn>) -> Self {
        Self { func }
    }

    pub fn range(range: impl SampleRange<u16> + Send + Sync + Clone + 'static) -> Self {
        Self::new(Arc::new(move |rng| rng.gen_range(range.clone())))
    }
}

impl Debug for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Count").finish()
    }
}

impl PlacementModifier for Count {
    fn process(
        &self,
        _chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        rng: &mut dyn rand::RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        vec![pos; (self.func)(rng) as _]
    }
}
