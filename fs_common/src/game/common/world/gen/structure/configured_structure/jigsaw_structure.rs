use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::game::common::{
    registry::RegistryID,
    world::{
        gen::structure::{
            piece::StructureNodeConfig, pool::StructurePool, Direction, StructureNode,
        },
        Position,
    },
};

use super::{ConfiguredStructurePlaceContext, ConfiguredStructurePlacer, StructureType};

pub struct JigsawFeatureType {}

impl StructureType for JigsawFeatureType {}

#[derive(Debug)]
pub struct ConfiguredJigsawFeature {
    pub start_pool: RegistryID<StructurePool>,
    pub depth: u8,
    // TODO: implement
    pub max_distance: u16,
    pub override_dir: Option<Direction>,
}

impl ConfiguredStructurePlacer for ConfiguredJigsawFeature {
    fn place(&self, x: i64, y: i64, ctx: ConfiguredStructurePlaceContext) {
        let mut hasher = DefaultHasher::new();
        x.hash(&mut hasher);
        y.hash(&mut hasher);
        let hashed = hasher.finish();

        let mut rng = StdRng::seed_from_u64(ctx.world_seed.wrapping_add(hashed));
        StructureNode::create_and_add(
            ctx.ecs,
            Position { x: x as _, y: y as _ },
            self.depth,
            self.max_distance,
            rng.gen(),
            StructureNodeConfig::new(self.start_pool.clone()),
            self.override_dir,
        );
    }
}
