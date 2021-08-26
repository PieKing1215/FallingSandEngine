
// use std::time::Duration;

// use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use falling_sand_engine::{common::{Settings, world::{Loader, Position, World}}, server::world::ServerChunk};
// use specs::{Builder, WorldExt};

// fn fibonacci(n: u64) -> u64 {
//     match n {
//         0 => 1,
//         1 => 1,
//         n => fibonacci(n-1) + fibonacci(n-2),
//     }
// }

// fn criterion_benchmark(c: &mut Criterion) {
//     c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));

//     let mut world_group = c.benchmark_group("world");
//     world_group.sample_size(50).sampling_mode(criterion::SamplingMode::Flat).measurement_time(Duration::from_secs_f32(10.0));
//     world_group.bench_function("generate", |b| {
//         b.iter(|| {
//             let mut w = World::<ServerChunk>::create(None);

//             let _loader = w.ecs.create_entity().with(Position{ x: 0.0, y: 0.0 }).with(Loader).build();

//             let settings = Settings {
//                 load_chunks: true,
//                 simulate_chunks: false,
//                 simulate_particles: false,
//                 ..Settings::default()
//             };

//             w.tick(0, &settings);
//             while !w.chunk_handler.load_queue.is_empty() {
//                 w.tick(0, &settings);
//             }
//         })
//     });

//     world_group.sample_size(100).sampling_mode(criterion::SamplingMode::Flat).measurement_time(Duration::from_secs_f32(5.0));
//     world_group.bench_function("simulate", |b| {
//         let mut w = World::<ServerChunk>::create(None);

//         let _loader = w.ecs.create_entity().with(Position{ x: 0.0, y: 0.0 }).with(Loader).build();

//         let mut settings = Settings {
//             load_chunks: true,
//             simulate_chunks: false,
//             simulate_particles: false,
//             ..Settings::default()
//         };

//         w.tick(0, &settings);
//         while !w.chunk_handler.load_queue.is_empty() {
//             w.tick(0, &settings);
//         }

//         settings.load_chunks = false;
//         settings.simulate_chunks = true;
//         settings.simulate_particles = true;

//         // println!("{} chunks loaded", w.chunk_handler.loaded_chunks.len());
        
//         b.iter(|| {
//             w.tick(0, &settings);
//         })
//     });
//     world_group.finish();

// }

// criterion_group!(benches, criterion_benchmark);
// criterion_main!(benches);