use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use specs::{
    Builder, Component, Entities, Entity, HashMapStorage, Join, System, WorldExt, WriteStorage,
};

use crate::game::common::world::{
    self, entity::Persistent, material::MaterialInstance, ChunkHandlerGeneric, ChunkState,
    Position, CHUNK_SIZE,
};

pub struct StructureNode {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
    pub generated: bool,
    pub depth: u8,
    pub rng: Box<dyn RngCore + Send + Sync>,
}

impl StructureNode {
    pub fn create_and_add(ecs: &mut specs::World, pos: Position, depth: u8, seed: i32) -> Entity {
        let rng = StdRng::seed_from_u64(seed as u64);
        let player = ecs
            .create_entity()
            .with(StructureNode {
                parent: None,
                children: vec![],
                generated: false,
                depth,
                rng: Box::new(rng),
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
}

fn is_finished(p: Entity, node_storage: &WriteStorage<StructureNode>) -> bool {
    let n = node_storage.get(p).unwrap();
    n.generated && n.children.iter().all(|c| is_finished(*c, node_storage))
}

impl<'a, H: ChunkHandlerGeneric + Send> System<'a> for UpdateStructureNodes<'a, H> {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, StructureNode>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, data: Self::SystemData) {
        profiling::scope!("UpdateAutoTargets::run");

        let (entities, mut node_storage, mut pos_storage) = data;

        let mut to_check = vec![];

        let to_add = (&entities, &mut node_storage, &mut pos_storage)
            .join()
            .flat_map(|(entity, node, pos)| {
                if node.generated {
                    if node.parent.is_none() {
                        to_check.push(entity);
                    } else if !entities.is_alive(node.parent.unwrap()) {
                        entities.delete(entity).unwrap();
                    }

                    return vec![];
                }

                let (chunk_x, chunk_y) = world::pixel_to_chunk_pos(pos.x as i64, pos.y as i64);
                let ch = self.chunk_handler.get_chunk(chunk_x, chunk_y);

                let Some(ch) = ch else {
                    return vec![];
                };

                if matches!(ch.get_state(), ChunkState::Cached | ChunkState::Active)
                    || matches!(ch.get_state(), ChunkState::Generating(n) if n >= 1)
                {
                    for dx in -((i64::from(node.depth) + 1) * 3)..=((i64::from(node.depth) + 1) * 3)
                    {
                        for dy in
                            -((i64::from(node.depth) + 1) * 3)..=((i64::from(node.depth) + 1) * 3)
                        {
                            self.chunk_handler
                                .set(
                                    pos.x as i64 + dx,
                                    pos.y as i64 + dy,
                                    MaterialInstance::air(),
                                )
                                .unwrap();
                        }
                    }

                    node.generated = true;

                    if node.depth > 0 {
                        let mut children = vec![];

                        for _ in 0..5 {
                            let rng = StdRng::seed_from_u64(node.rng.gen());
                            children.push((
                                entity,
                                StructureNode {
                                    parent: Some(entity),
                                    children: vec![],
                                    generated: false,
                                    depth: node.depth - 1,
                                    rng: Box::new(rng),
                                },
                                Position {
                                    x: pos.x
                                        + node.rng.gen_range(
                                            -f64::from(CHUNK_SIZE)..=f64::from(CHUNK_SIZE),
                                        ),
                                    y: pos.y
                                        + node.rng.gen_range(
                                            -f64::from(CHUNK_SIZE)..=f64::from(CHUNK_SIZE),
                                        ),
                                },
                            ));
                        }

                        return children;
                    }
                }

                vec![]
            })
            .collect::<Vec<_>>();

        for (parent, node, p) in to_add {
            let c = entities
                .build_entity()
                .with(node, &mut node_storage)
                .with(p, &mut pos_storage)
                .build();

            node_storage.get_mut(parent).unwrap().children.push(c);
        }

        for p in to_check {
            if is_finished(p, &node_storage) {
                entities.delete(p).unwrap();
            }
        }
    }
}
