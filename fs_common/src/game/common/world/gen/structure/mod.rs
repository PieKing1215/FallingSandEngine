pub mod registry;
pub mod structure;

use std::sync::Arc;

use rand::{
    distributions::Standard, prelude::Distribution, rngs::StdRng, seq::SliceRandom, Rng, RngCore,
    SeedableRng,
};
use specs::{
    Builder, Component, Entities, Entity, HashMapStorage, Join, System, WorldExt, WriteStorage,
};

use crate::game::{
    common::{
        world::{
            self,
            entity::Persistent,
            gen::structure::structure::StructureNodeConfig,
            material::{self, color::Color, MaterialInstance, PhysicsType},
            ChunkHandlerGeneric, ChunkState, Position,
        },
        Rect,
    },
    Registries,
};

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
    pub fn rotated(self, angle: AngleMod) -> Self {
        match angle {
            AngleMod::None => self,
            AngleMod::Clockwise90 => match self {
                Direction::Up => Direction::Right,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Up,
                Direction::Right => Direction::Down,
            },
            AngleMod::CounterClockwise90 => match self {
                Direction::Up => Direction::Left,
                Direction::Down => Direction::Right,
                Direction::Left => Direction::Down,
                Direction::Right => Direction::Up,
            },
            AngleMod::Angle180 => self.opposite(),
        }
    }

    pub fn angle(self, other: Self) -> AngleMod {
        // TODO: there's probably a better way to implement this

        if self == other {
            return AngleMod::None;
        }

        if other
            == match self {
                Direction::Up => Direction::Right,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Up,
                Direction::Right => Direction::Down,
            }
        {
            return AngleMod::Clockwise90;
        }

        if other
            == match self {
                Direction::Up => Direction::Left,
                Direction::Down => Direction::Right,
                Direction::Left => Direction::Down,
                Direction::Right => Direction::Up,
            }
        {
            return AngleMod::CounterClockwise90;
        }

        AngleMod::Angle180
    }
}

// TODO: think of better names
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AngleMod {
    None,
    Clockwise90,
    CounterClockwise90,
    Angle180,
}

impl AngleMod {
    pub fn degrees(&self) -> f32 {
        match self {
            AngleMod::None => 0.0,
            AngleMod::Clockwise90 => 90.0,
            AngleMod::CounterClockwise90 => -90.0,
            AngleMod::Angle180 => 180.0,
        }
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
        seed: i32,
        config: StructureNodeConfig,
    ) -> Entity {
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let player = ecs
            .create_entity()
            .with(StructureNode {
                parent: None,
                children: vec![],
                generated: None,
                depth,
                direction: rng.gen(),
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

pub struct UpdateStructureNodes<'a, H: ChunkHandlerGeneric + Send> {
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

impl<'a, H: ChunkHandlerGeneric + Send> System<'a> for UpdateStructureNodes<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, StructureNode>,
        WriteStorage<'a, Position>,
    );

    #[allow(clippy::too_many_lines)]
    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateAutoTargets::run");

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
            let ch = self.chunk_handler.get_chunk(chunk_x, chunk_y);

            let Some(ch) = ch else {
                // log::trace!("add {entity:?}");
                node_storage.insert(entity, node).unwrap();
                pos_storage.insert(entity, pos).unwrap();
                continue;
            };

            if matches!(ch.get_state(), ChunkState::Cached | ChunkState::Active)
                || matches!(ch.get_state(), ChunkState::Generating(n) if n >= 2)
            {
                node_storage.insert(entity, node).unwrap();
                pos_storage.insert(entity, pos).unwrap();
                let root = root(entity, node_storage.get(entity).unwrap(), &node_storage);
                let all_bounds = all_bounds(root.1, &node_storage);
                node = node_storage.remove(entity).unwrap();
                pos = pos_storage.remove(entity).unwrap();

                node.generated = Some(Err(()));

                // try every structure in desired pool
                let mut pool = self
                    .registries
                    .structure_pools
                    .get(&node.config.pool)
                    .unwrap()
                    .clone();
                pool.shuffle(&mut node.rng);
                'outer: for pool_structure in pool {
                    let mut opts =
                        pool_structure.options((pos.x as i64, pos.y as i64), node.direction);
                    opts.shuffle(&mut node.rng);

                    // log::debug!("{} {:?} {:?}", opts.len(), node.direction, node.parent);

                    // try every connection in structure
                    for o in opts {
                        // log::debug!("{o:?}");

                        let (bounds, children) = o;

                        let ok = !all_bounds
                            .iter()
                            .any(|r| r.inflated(-1).intersects(&bounds));

                        if ok {
                            // TODO
                            // structure.place(self.chunk_handler, pos.x as i64, pos.y as i64);
                            for x in bounds.left()..=bounds.right() {
                                for y in bounds.top()..=bounds.bottom() {
                                    let m = *self.chunk_handler.get(x, y).unwrap();

                                    self.chunk_handler
                                        .set(
                                            x,
                                            y,
                                            MaterialInstance {
                                                material_id: material::COBBLE_STONE,
                                                physics: PhysicsType::Solid,
                                                color: Color::rgb(
                                                    (m.color.r_f32() + 1.0) / 2.0,
                                                    m.color.g_f32(),
                                                    m.color.b_f32(),
                                                ),
                                            },
                                        )
                                        .unwrap();
                                }
                            }

                            node.generated = Some(Ok(StructureNodeGenData { bounds }));

                            let mut children = children
                                .into_iter()
                                .filter(|(_, config)| node.depth > 0 || config.depth_override)
                                .map(|(placement, config)| {
                                    let rng = StdRng::seed_from_u64(node.rng.gen());
                                    (
                                        entity,
                                        StructureNode {
                                            parent: Some(entity),
                                            children: vec![],
                                            generated: None,
                                            depth: if node.depth == 0 { 0 } else { node.depth - 1 },
                                            rng: Box::new(rng),
                                            direction: placement.direction_out,
                                            config,
                                        },
                                        Position { x: placement.x as _, y: placement.y as _ },
                                    )
                                })
                                .collect();

                            to_add.append(&mut children);

                            break 'outer;
                        }
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
