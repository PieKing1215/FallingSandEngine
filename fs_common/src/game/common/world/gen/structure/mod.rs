use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use specs::{
    Builder, Component, Entities, Entity, HashMapStorage, Join, System, WorldExt, WriteStorage,
};

use crate::game::common::{
    world::{
        self,
        entity::Persistent,
        material::{self, color::Color, MaterialInstance, PhysicsType},
        ChunkHandlerGeneric, ChunkState, Position,
    },
    Rect,
};

pub struct StructureNode {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
    pub generated: Option<Result<StructureNodeGenData, ()>>,
    pub depth: u8,
    pub rng: Box<dyn RngCore + Send + Sync>,
}

pub struct StructureNodeGenData {
    pub bounds: Rect<f64>,
}

impl StructureNode {
    pub fn create_and_add(ecs: &mut specs::World, pos: Position, depth: u8, seed: i32) -> Entity {
        let rng = StdRng::seed_from_u64(seed as u64);
        let player = ecs
            .create_entity()
            .with(StructureNode {
                parent: None,
                children: vec![],
                generated: None,
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
    n.generated.is_some() && n.children.iter().all(|c| is_finished(*c, node_storage))
}

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

fn all_bounds(node: &StructureNode, node_storage: &WriteStorage<StructureNode>) -> Vec<Rect<f64>> {
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

                // 4 placement attempts
                for _ in 0..4 {
                    let w = node.rng.gen_range(100.0..=200.0);
                    let h = node.rng.gen_range(25.0..=200.0);
                    let bounds = Rect::new(pos.x, pos.y - h / 2.0, pos.x + w, pos.y + h / 2.0);

                    let ok = !all_bounds
                        .iter()
                        .any(|r| r.inflated(-0.1).intersects(&bounds));

                    if ok {
                        for x in bounds.left() as i64..=bounds.right() as i64 {
                            for y in bounds.top() as i64..=bounds.bottom() as i64 {
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

                        if node.depth > 0 {
                            let mut children = vec![];

                            for _ in 0..5 {
                                let rng = StdRng::seed_from_u64(node.rng.gen());
                                children.push((
                                    entity,
                                    StructureNode {
                                        parent: Some(entity),
                                        children: vec![],
                                        generated: None,
                                        depth: node.depth - 1,
                                        rng: Box::new(rng),
                                    },
                                    Position {
                                        x: bounds.right(),
                                        y: node.rng.gen_range(bounds.range_tb()),
                                    },
                                ));
                            }

                            to_add.append(&mut children);
                        }

                        break;
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

        for p in to_check {
            if is_finished(p, &node_storage) {
                entities.delete(p).unwrap();
            }
        }
    }
}
