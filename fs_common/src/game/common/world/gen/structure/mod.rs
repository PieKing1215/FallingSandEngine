pub mod configured_structure;
pub mod piece;
pub mod pool;
pub mod set;

use std::sync::Arc;

use rand::{
    distributions::Standard, prelude::Distribution, rngs::StdRng, seq::SliceRandom, Rng, RngCore,
    SeedableRng,
};
use specs::{
    Builder, Component, Entities, Entity, HashMapStorage, Join, System, WorldExt, WriteStorage,
};

use crate::game::common::{
    registry::RegistryID,
    world::{
        self, chunk_access::FSChunkAccess, entity::Persistent,
        gen::structure::piece::StructureNodeConfig, ChunkState, Position,
    },
    Rect, Registries,
};

use self::piece::StructurePiece;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    #[must_use]
    pub fn others(self) -> [Self; 3] {
        match self {
            Direction::Up => [Direction::Down, Direction::Left, Direction::Right],
            Direction::Down => [Direction::Up, Direction::Left, Direction::Right],
            Direction::Left => [Direction::Up, Direction::Down, Direction::Right],
            Direction::Right => [Direction::Up, Direction::Down, Direction::Left],
        }
    }

    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    #[must_use]
    pub fn rotated(self, angle: AngleDiff) -> Self {
        match angle {
            AngleDiff::None => self,
            AngleDiff::Clockwise90 => match self {
                Direction::Up => Direction::Right,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Up,
                Direction::Right => Direction::Down,
            },
            AngleDiff::CounterClockwise90 => match self {
                Direction::Up => Direction::Left,
                Direction::Down => Direction::Right,
                Direction::Left => Direction::Down,
                Direction::Right => Direction::Up,
            },
            AngleDiff::Angle180 => self.opposite(),
        }
    }

    pub fn angle(self, other: Self) -> AngleDiff {
        // TODO: there's probably a better way to implement this

        if self == other {
            return AngleDiff::None;
        }

        if other
            == match self {
                Direction::Up => Direction::Right,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Up,
                Direction::Right => Direction::Down,
            }
        {
            return AngleDiff::Clockwise90;
        }

        if other
            == match self {
                Direction::Up => Direction::Left,
                Direction::Down => Direction::Right,
                Direction::Left => Direction::Down,
                Direction::Right => Direction::Up,
            }
        {
            return AngleDiff::CounterClockwise90;
        }

        AngleDiff::Angle180
    }

    pub fn vec(&self) -> (i8, i8) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AngleDiff {
    None,
    Clockwise90,
    CounterClockwise90,
    Angle180,
}

impl AngleDiff {
    pub fn degrees(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Clockwise90 => 90.0,
            Self::CounterClockwise90 => -90.0,
            Self::Angle180 => 180.0,
        }
    }

    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::None => Self::Angle180,
            Self::Clockwise90 => Self::CounterClockwise90,
            Self::CounterClockwise90 => Self::Clockwise90,
            Self::Angle180 => Self::None,
        }
    }

    #[inline]
    pub fn rotate_point(&self, point: (i64, i64), pivot: (i64, i64)) -> (i64, i64) {
        let sin = self.degrees().to_radians().sin();
        let cos = self.degrees().to_radians().cos();
        (
            (cos * (point.0 - pivot.0) as f32 - sin * (point.1 - pivot.1) as f32 + pivot.0 as f32)
                as i64,
            (sin * (point.0 - pivot.0) as f32 + cos * (point.1 - pivot.1) as f32 + pivot.1 as f32)
                as i64,
        )
    }
}

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..=3) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

pub struct StructureNode {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
    pub generated: Option<Result<StructureNodeGenData, ()>>,
    pub depth: u8,
    pub max_distance: u16,
    pub rng: Box<dyn RngCore + Send + Sync>,
    /// Direction to parent
    pub direction: Direction,
    pub config: StructureNodeConfig,
}

pub struct StructureNodeGenData {
    pub bounds: Rect<i64>,
}

impl StructureNode {
    pub fn create_and_add(
        ecs: &mut specs::World,
        pos: Position,
        depth: u8,
        max_distance: u16,
        seed: i32,
        config: StructureNodeConfig,
        override_dir: Option<Direction>,
    ) -> Entity {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let player = ecs
            .create_entity()
            .with(StructureNode {
                parent: None,
                children: vec![],
                generated: None,
                depth,
                max_distance,
                direction: override_dir.unwrap_or_else(|| rng.gen()),
                rng: Box::new(rng),
                config,
            })
            .with(Persistent)
            .with(pos)
            .build();

        player
    }
}

impl Component for StructureNode {
    type Storage = HashMapStorage<Self>;
}

pub struct UpdateStructureNodes<'a, H: FSChunkAccess + Send> {
    pub chunk_handler: &'a mut H,
    pub registries: Arc<Registries>,
}

// fn is_finished(p: Entity, node_storage: &WriteStorage<StructureNode>) -> bool {
//     let n = node_storage.get(p).unwrap();
//     n.generated.is_some() && n.children.iter().all(|c| is_finished(*c, node_storage))
// }

fn root<'a>(
    e: Entity,
    node: &'a StructureNode,
    node_storage: &'a WriteStorage<StructureNode>,
) -> (Entity, &'a StructureNode) {
    if let Some(p) = node.parent {
        root(p, node_storage.get(p).unwrap(), node_storage)
    } else {
        (e, node)
    }
}

fn all_bounds(node: &StructureNode, node_storage: &WriteStorage<StructureNode>) -> Vec<Rect<i64>> {
    let mut v = vec![];
    if let Some(Ok(gen)) = &node.generated {
        v.push(gen.bounds);
    }

    // log::trace!("{:?}", node.children);
    // log::trace!("{:?}", node.children.iter().map(|c| node_storage.get(*c).is_some()).collect::<Vec<_>>());
    for c in &node.children {
        v.append(&mut all_bounds(node_storage.get(*c).unwrap(), node_storage));
    }

    v
}

impl<'a, H: FSChunkAccess + Send> System<'a> for UpdateStructureNodes<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, StructureNode>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateStructureNodes::run");

        let (entities, mut node_storage, mut pos_storage) = data;

        let mut to_check = vec![];

        let all = (&entities, &mut node_storage, &mut pos_storage)
            .join()
            .map(|(e, _, _)| e)
            .collect::<Vec<_>>();

        let mut to_add = vec![];

        for entity in all {
            let mut node = node_storage.remove(entity).unwrap();
            let mut pos = pos_storage.remove(entity).unwrap();
            // log::trace!("remove {entity:?}");

            if node.generated.is_some() {
                if node.parent.is_none() {
                    to_check.push(entity);
                } else if !entities.is_alive(node.parent.unwrap()) {
                    entities.delete(entity).unwrap();
                }

                // log::trace!("add {entity:?}");
                node_storage.insert(entity, node).unwrap();
                pos_storage.insert(entity, pos).unwrap();
                continue;
            }

            let (chunk_x, chunk_y) = world::pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
            let ch = self.chunk_handler.chunk_at_dyn((chunk_x, chunk_y));

            let Some(ch) = ch else {
                // log::trace!("add {entity:?}");
                node_storage.insert(entity, node).unwrap();
                pos_storage.insert(entity, pos).unwrap();
                continue;
            };

            if matches!(ch.state(), ChunkState::Cached | ChunkState::Active)
                || matches!(ch.state(), ChunkState::Generating(n) if n >= 2)
            {
                node_storage.insert(entity, node).unwrap();
                pos_storage.insert(entity, pos).unwrap();
                let root = root(entity, node_storage.get(entity).unwrap(), &node_storage);
                let root_pos = pos_storage.get(root.0).unwrap().clone();
                let all_bounds = all_bounds(root.1, &node_storage);
                node = node_storage.remove(entity).unwrap();
                pos = pos_storage.remove(entity).unwrap();

                node.generated = Some(Err(()));

                // try every structure in desired pool
                let mut pool = self
                    .registries
                    .structure_pools
                    .get(&node.config.pool)
                    .expect(format!("Missing structure pool {:?}", node.config.pool).as_str())
                    .pool
                    .clone();
                pool.shuffle(&mut node.rng);

                // try placing normal pool
                if let Some(mut children) = self.place(
                    &pool,
                    &pos,
                    &mut node,
                    &all_bounds,
                    &root_pos,
                    entity,
                    false,
                ) {
                    to_add.append(&mut children);
                } else if let Some(fallback_pool) = &node.config.fallback_pool {
                    // if normal pool failed, try placing fallback pool
                    let mut fallback_pool = self
                        .registries
                        .structure_pools
                        .get(fallback_pool)
                        .unwrap()
                        .pool
                        .clone();
                    fallback_pool.shuffle(&mut node.rng);

                    if let Some(mut children) = self.place(
                        &fallback_pool,
                        &pos,
                        &mut node,
                        &all_bounds,
                        &root_pos,
                        entity,
                        true,
                    ) {
                        to_add.append(&mut children);
                    }
                }
            }

            // log::trace!("add {entity:?}");
            node_storage.insert(entity, node).unwrap();
            pos_storage.insert(entity, pos).unwrap();
        }

        for (parent, node, p) in to_add {
            let c = entities
                .build_entity()
                .with(node, &mut node_storage)
                .with(p, &mut pos_storage)
                .build();

            node_storage.get_mut(parent).unwrap().children.push(c);
        }

        // for p in to_check {
        //     if is_finished(p, &node_storage) {
        //         entities.delete(p).unwrap();
        //     }
        // }
    }
}

impl<H: FSChunkAccess + Send> UpdateStructureNodes<'_, H> {
    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        pool: &[RegistryID<StructurePiece>],
        pos: &Position,
        node: &mut StructureNode,
        all_bounds: &[Rect<i64>],
        root_pos: &Position,
        entity: Entity,
        ignore_restrictions: bool,
    ) -> Option<Vec<(Entity, StructureNode, Position)>> {
        // for every structure piece in the pool
        for pool_structure in pool
            .iter()
            .map(|k| self.registries.structure_pieces.get(k).unwrap())
        {
            let mut opts = pool_structure.options((pos.x as i64, pos.y as i64), node.direction);
            opts.shuffle(&mut node.rng);

            // try every connection in structure
            for o in opts {
                let (bounds, children, place_fn) = o;

                let ok = ignore_restrictions
                    || !all_bounds
                        .iter()
                        .any(|r| r.inflated(-1).intersects(&bounds));

                if ok {
                    place_fn(pool_structure, self.chunk_handler).unwrap();

                    node.generated = Some(Ok(StructureNodeGenData { bounds }));

                    let children = children
                        .into_iter()
                        .filter(|(pos, config)| {
                            (node.depth > 0 || config.depth_override) && {
                                let dx = root_pos.x as i64 - pos.x;
                                let dy = root_pos.y as i64 - pos.y;
                                dx * dx + dy * dy
                                    < (i64::from(node.max_distance) * i64::from(node.max_distance))
                            }
                        })
                        .map(|(placement, config)| {
                            let rng = StdRng::seed_from_u64(node.rng.gen());
                            (
                                entity,
                                StructureNode {
                                    parent: Some(entity),
                                    children: vec![],
                                    generated: None,
                                    depth: if node.depth == 0 { 0 } else { node.depth - 1 },
                                    max_distance: node.max_distance,
                                    rng: Box::new(rng),
                                    direction: placement.direction_out,
                                    config,
                                },
                                Position { x: placement.x as _, y: placement.y as _ },
                            )
                        })
                        .collect();

                    return Some(children);
                }
            }
        }

        None
    }
}
