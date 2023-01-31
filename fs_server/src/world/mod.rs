mod chunk;

pub use chunk::*;

#[cfg(test)]
mod tests {
    use fs_common::game::common::world::physics::Physics;
    use fs_common::game::common::world::{
        chunk_index, ChunkHandler, ChunkHandlerGeneric, Loader, Position,
    };
    use fs_common::game::common::Settings;
    use fs_common::game::Registries;

    use fs_common::game::common::world::{gen::TestGenerator, FilePersistent, Velocity};
    use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};
    use specs::{Builder, WorldExt};

    use crate::world::ServerChunk;

    #[test]
    fn chunk_loading() {
        let registries = Registries::empty();

        let mut ch: ChunkHandler<ServerChunk> =
            ChunkHandler::<ServerChunk>::new(TestGenerator::new(), None);

        assert_eq!(ch.load_queue.len(), 0);
        assert_eq!(ch.loaded_chunks.len(), 0);

        // queue a chunk
        let queued_1 = ch.queue_load_chunk(11, -12);

        assert!(queued_1);
        assert_eq!(ch.load_queue.len(), 1);
        assert_eq!(ch.loaded_chunks.len(), 0);

        // queue the same chunk
        // should fail since it's already queued
        let queued_1_again = ch.queue_load_chunk(11, -12);

        assert!(!queued_1_again);
        assert_eq!(ch.load_queue.len(), 1);
        assert_eq!(ch.loaded_chunks.len(), 0);

        // queue a different chunk
        let queued_2 = ch.queue_load_chunk(-3, 2);

        assert!(queued_2);
        assert_eq!(ch.load_queue.len(), 2);
        assert_eq!(ch.loaded_chunks.len(), 0);

        assert!(!ch.is_chunk_loaded(11, -12));
        assert!(!ch.is_chunk_loaded(-3, 2));

        // do a few ticks to load some chunks
        let mut ecs = specs::World::new();
        ecs.register::<SimpleMarker<FilePersistent>>();
        ecs.insert(SimpleMarkerAllocator::<FilePersistent>::default());
        ecs.register::<Position>();
        ecs.register::<Velocity>();
        ecs.register::<Loader>();

        let loader = ecs
            .create_entity()
            .with(Position { x: 110.0, y: -120.0 })
            .with(Loader)
            .build();

        let mut phys = Physics::new();

        ch.tick(0, &Settings::default(), &mut ecs, &mut phys, &registries, 2);
        while !ch.load_queue.is_empty() {
            ch.tick(0, &Settings::default(), &mut ecs, &mut phys, &registries, 2);
        }

        assert!(ch.is_chunk_loaded(11, -12));
        assert!(ch.is_chunk_loaded(-3, 2));
        assert!(!ch.is_chunk_loaded(120, -120));
        assert!(!ch.is_chunk_loaded(30, 20));

        let index_1 = chunk_index(11, -12);
        let loaded_1 = ch
            .loaded_chunks
            .iter()
            .any(|(&i, c)| i == index_1 && c.chunk_x == 11 && c.chunk_y == -12);
        assert!(loaded_1);
        assert!(ch.get_chunk(11, -12).is_some());

        let index_2 = chunk_index(-3, 2);
        let loaded_2 = ch
            .loaded_chunks
            .iter()
            .any(|(&i, c)| i == index_2 && c.chunk_x == -3 && c.chunk_y == 2);
        assert!(loaded_2);
        assert!(ch.get_chunk(-3, 2).is_some());

        assert!(ch.get_chunk(0, 0).is_some());
        assert!(ch.get_chunk(-11, -12).is_none());
        assert!(ch.get_chunk(30, -2).is_none());
        assert!(ch.get_chunk(-3, 30).is_none());
        assert!(ch.get_chunk(-120, 11).is_none());

        // should unload since no loaders are nearby
        assert_eq!(ecs.delete_entity(loader), Ok(()));
        ch.tick(0, &Settings::default(), &mut ecs, &mut phys, &registries, 2);

        assert!(!ch.is_chunk_loaded(11, -12));
        assert!(!ch.is_chunk_loaded(-3, 2));
    }

    #[test]
    fn zones() {
        let ch: ChunkHandler<ServerChunk> =
            ChunkHandler::<ServerChunk>::new(TestGenerator::new(), None);

        let center = (12.3, -42.2);
        let screen = ch.get_screen_zone(center);
        let active = ch.get_active_zone(center);
        let load = ch.get_load_zone(center);
        let unload = ch.get_unload_zone(center);

        assert!(screen.width() <= active.width() && screen.height() <= active.height());
        assert!(active.width() < load.width() && active.height() < load.height());
        assert!(load.width() < unload.width() && load.height() < unload.height());
    }
}
