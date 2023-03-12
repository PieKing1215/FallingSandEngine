mod chunk;

pub use chunk::*;

#[cfg(test)]
mod tests {
    use chunksystem::ChunkQuery;
    use fs_common::game::common::world::chunk_access::FSChunkAccess;
    use fs_common::game::common::world::physics::Physics;
    use fs_common::game::common::world::{self, ChunkHandler, ChunkTickContext, Loader, Position};
    use fs_common::game::common::Settings;
    use fs_common::game::common::{FileHelper, Registries};

    use fs_common::game::common::world::gen::TestGenerator;
    use specs::{Builder, WorldExt};

    use crate::world::ServerChunk;

    #[test]
    fn chunk_loading() {
        let registries = std::sync::Arc::new(Registries::empty());
        let file_helper = FileHelper::new("./gamedir/".into(), "./gamedir/assets/".into());

        let mut ch: ChunkHandler<ServerChunk> =
            ChunkHandler::<ServerChunk>::new(TestGenerator::new(), None);

        assert_eq!(ch.load_queue.len(), 0);
        assert_eq!(ch.manager.len(), 0);

        // queue a chunk
        let queued_1 = ch.queue_load_chunk(11, -12);

        assert!(queued_1);
        assert_eq!(ch.load_queue.len(), 1);
        assert_eq!(ch.manager.len(), 0);

        // queue the same chunk
        // should fail since it's already queued
        let queued_1_again = ch.queue_load_chunk(11, -12);

        assert!(!queued_1_again);
        assert_eq!(ch.load_queue.len(), 1);
        assert_eq!(ch.manager.len(), 0);

        // queue a different chunk
        let queued_2 = ch.queue_load_chunk(-3, 2);

        assert!(queued_2);
        assert_eq!(ch.load_queue.len(), 2);
        assert_eq!(ch.manager.len(), 0);

        assert!(!ch.is_chunk_loaded((11, -12)));
        assert!(!ch.is_chunk_loaded((-3, 2)));

        // do a few ticks to load some chunks
        let mut ecs = world::ecs();

        let loader = ecs
            .create_entity()
            .with(Position { x: 110.0, y: -120.0 })
            .with(Loader)
            .build();

        let mut phys = Physics::new();

        ch.tick(ChunkTickContext {
            tick_time: 0,
            settings: &Settings::default(),
            world: &mut ecs,
            physics: &mut phys,
            registries: &registries,
            seed: 2,
            file_helper: &file_helper,
        });
        while !ch.load_queue.is_empty() {
            ch.tick(ChunkTickContext {
                tick_time: 0,
                settings: &Settings::default(),
                world: &mut ecs,
                physics: &mut phys,
                registries: &registries,
                seed: 2,
                file_helper: &file_helper,
            });
        }

        assert!(ch.is_chunk_loaded((11, -12)));
        assert!(ch.is_chunk_loaded((-3, 2)));
        assert!(!ch.is_chunk_loaded((120, -120)));
        assert!(!ch.is_chunk_loaded((30, 20)));

        let index_1 = (11, -12);
        let loaded_1 = unsafe { ch.manager.raw().iter() }
            .any(|(&i, c)| i == index_1 && c.chunk_x() == 11 && c.chunk_y() == -12);
        assert!(loaded_1);
        assert!(ch.chunk_at_dyn((11, -12)).is_some());

        let index_2 = (-3, 2);
        let loaded_2 = unsafe { ch.manager.raw().iter() }
            .any(|(&i, c)| i == index_2 && c.chunk_x() == -3 && c.chunk_y() == 2);
        assert!(loaded_2);
        assert!(ch.chunk_at_dyn((-3, 2)).is_some());

        assert!(ch.chunk_at_dyn((0, 0)).is_some());
        assert!(ch.chunk_at_dyn((-11, -12)).is_some());
        assert!(ch.chunk_at_dyn((-11, -20)).is_none());
        assert!(ch.chunk_at_dyn((30, -2)).is_none());
        assert!(ch.chunk_at_dyn((-3, 30)).is_none());
        assert!(ch.chunk_at_dyn((-120, 11)).is_none());

        // should unload since no loaders are nearby
        assert_eq!(ecs.delete_entity(loader), Ok(()));
        ch.tick(ChunkTickContext {
            tick_time: 0,
            settings: &Settings::default(),
            world: &mut ecs,
            physics: &mut phys,
            registries: &registries,
            seed: 2,
            file_helper: &file_helper,
        });

        assert!(!ch.is_chunk_loaded((11, -12)));
        assert!(!ch.is_chunk_loaded((-3, 2)));
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
