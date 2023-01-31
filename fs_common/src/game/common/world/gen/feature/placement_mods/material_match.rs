use std::sync::Arc;

use crate::game::{
    common::world::{
        gen::{feature::PlacementModifier, populator::ChunkContext},
        material::{self, MaterialID, MaterialInstance, PhysicsType},
    },
    Registries,
};

pub type MaterialMatchFn = dyn Fn(&MaterialInstance) -> bool + Send + Sync;

pub struct MaterialMatch {
    predicate: Arc<MaterialMatchFn>,
}

impl MaterialMatch {
    pub fn new(predicate: Arc<MaterialMatchFn>) -> Self {
        Self { predicate }
    }

    pub fn non_air() -> Self {
        Self::new(Arc::new(|m| m.material_id != material::AIR))
    }

    pub fn physics(typ: PhysicsType) -> Self {
        Self::new(Arc::new(move |m| m.physics == typ))
    }

    pub fn material(mat: MaterialID) -> Self {
        Self::new(Arc::new(move |m| m.material_id == mat))
    }
}

impl std::fmt::Debug for MaterialMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaterialMatch").finish()
    }
}

impl PlacementModifier for MaterialMatch {
    fn process(
        &self,
        chunks: &mut ChunkContext<1>,
        pos: (i32, i32),
        _seed: i32,
        _rng: &mut dyn rand::RngCore,
        _registries: &Registries,
    ) -> Vec<(i32, i32)> {
        if (self.predicate)(chunks.get(pos.0, pos.1).unwrap()) {
            vec![pos]
        } else {
            vec![]
        }
    }
}
