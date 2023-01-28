use std::{fs::File, str::FromStr, thread};

use backtrace::Backtrace;
use clap::{crate_authors, crate_name, crate_version, Arg, Command};
use fs_client::{render::Renderer, world::ClientWorld, ClientGame};
use fs_common::game::common::{
    world::{
        entity::{
            GameEntity, Hitbox, Persistent, PhysicsEntity, Player, PlayerClipboard,
            PlayerMovementMode,
        },
        physics::PHYSICS_SCALE,
        AutoTarget, Camera, CollisionFlags, Loader, Position, RigidBodyComponent, Target,
        TargetStyle, Velocity,
    },
    FileHelper,
};
use fs_server::ServerGame;
use log::{error, info, LevelFilter};
use rapier2d::{
    na::{Isometry2, Vector2},
    prelude::{ColliderBuilder, InteractionGroups, RigidBodyBuilder},
};
use salva2d::{integrations::rapier::ColliderSampling, object::Boundary};
use simplelog::{CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};
use specs::{Builder, WorldExt};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

#[allow(clippy::needless_pass_by_value)]
fn is_type<T: FromStr>(val: &str) -> Result<(), String>
where
    <T as std::str::FromStr>::Err: std::string::ToString,
{
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[profiling::function]
pub fn main() -> Result<(), String> {
    unsafe {
        fs_client::render::BUILD_DATETIME = option_env!("BUILD_DATETIME");
        fs_client::render::GIT_HASH = option_env!("GIT_HASH");
    }

    let matches = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debugging features"),
        )
        .arg(
            Arg::new("no-tick")
                .long("no-tick")
                .help("Turn off simulation by default"),
        )
        .arg(
            Arg::new("connect")
                .short('c')
                .long("connect")
                .takes_value(true)
                .value_name("IP:PORT")
                .help("Connect to a server automatically"),
        )
        .arg(
            Arg::new("game-dir")
                .long("game-dir")
                .takes_value(true)
                .value_name("PATH")
                .default_value("./gamedir/")
                .help("Set the game directory"),
        )
        .arg(
            Arg::new("assets-dir")
                .long("assets-dir")
                .takes_value(true)
                .value_name("PATH")
                .default_value("./gamedir/assets/")
                .help("Set the assets directory"),
        )
        .subcommand(
            Command::new("server").about("Run dedicated server").arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .takes_value(true)
                    .value_name("PORT")
                    .default_value("6673")
                    .validator(is_type::<u16>)
                    .help("The port to run the server on"),
            ),
        )
        .get_matches();

    let file_helper = FileHelper::new(
        matches.value_of("game-dir").unwrap().into(),
        matches.value_of("assets-dir").unwrap().into(),
    );

    if !file_helper.game_path("").exists() {
        info!("game dir missing, creating it...");
        std::fs::create_dir_all(file_helper.game_path("")).expect("Failed to create game dir:");
    }

    if !file_helper.asset_path("").exists() {
        info!("asset dir missing, creating it...");
        std::fs::create_dir_all(file_helper.asset_path("")).expect("Failed to create asset dir:");
    }

    let server = matches.subcommand_matches("server").is_some();
    let client = !server;

    std::env::set_var("RAYON_NUM_THREADS", format!("{}", num_cpus::get() - 1));

    if server {
        crossterm::terminal::enable_raw_mode().unwrap();

        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);

        let mut terminal = Terminal::new(backend).unwrap();

        std::panic::set_hook(Box::new(|info| {
            let stdout = std::io::stdout();
            let mut backend = CrosstermBackend::new(stdout);
            backend.clear().unwrap();
            backend.set_cursor(0, 0).unwrap();

            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");
            let bt = Backtrace::new();

            let mut c: tui::buffer::Cell = tui::buffer::Cell::default();
            c.set_symbol(
                format!(
                    "thread '{name}' {info}\nSee latest.log for more details. Backtrace:\n{bt:?}\n"
                )
                .as_str(),
            );
            c.set_fg(tui::style::Color::LightRed);
            let t: (u16, u16, _) = (0, 0, &c);

            backend.draw(std::iter::once(t)).unwrap();
            backend.flush().unwrap();

            error!("thread '{}' {}", name, info);
            error!(
                "See server_latest.log for more details. Backtrace:\n{:?}",
                bt
            );
        }));

        let res = std::panic::catch_unwind(move || {
            println!("Starting server...");
            let mut game: ServerGame = ServerGame::new(file_helper);

            if let Some(w) = &mut game.0.world {
                let rigid_body = RigidBodyBuilder::new_dynamic()
                    .position(Isometry2::new(Vector2::new(0.0, 20.0), 0.0))
                    .lock_rotations()
                    .gravity_scale(0.0)
                    .build();
                let handle = w.physics.bodies.insert(rigid_body);
                let collider =
                    ColliderBuilder::cuboid(12.0 / PHYSICS_SCALE / 2.0, 20.0 / PHYSICS_SCALE / 2.0)
                        .collision_groups(InteractionGroups::new(
                            CollisionFlags::PLAYER.bits(),
                            (CollisionFlags::RIGIDBODY | CollisionFlags::ENTITY).bits(),
                        ))
                        .density(1.5)
                        .friction(0.3)
                        .build();
                let co_handle =
                    w.physics
                        .colliders
                        .insert_with_parent(collider, handle, &mut w.physics.bodies);
                let bo_handle = w
                    .physics
                    .fluid_pipeline
                    .liquid_world
                    .add_boundary(Boundary::new(Vec::new()));
                w.physics.fluid_pipeline.coupling.register_coupling(
                    bo_handle,
                    co_handle,
                    ColliderSampling::DynamicContactSampling,
                );

                let _player = w
                    .ecs
                    .create_entity()
                    .with(Player {
                        movement: PlayerMovementMode::Free,
                        clipboard: PlayerClipboard::default(),
                    })
                    .with(GameEntity)
                    .with(PhysicsEntity {
                        on_ground: false,
                        gravity: 0.2,
                        edge_clip_distance: 2.0,
                        collision: true,
                        collide_with_sand: true,
                    })
                    .with(Persistent)
                    .with(Position { x: 0.0, y: -20.0 })
                    .with(Velocity { x: 0.0, y: 0.0 })
                    .with(Hitbox { x1: -6.0, y1: -10.0, x2: 6.0, y2: 10.0 })
                    .with(Loader)
                    .with(RigidBodyComponent::of(handle))
                    .build();
            };

            println!("Starting main loop...");
            match game.run(&matches, &mut terminal) {
                Ok(_) => {},
                Err(e) => panic!("Server encountered a fatal error: {e}"),
            }
        });

        if res.is_err() {
            println!("Server crashed, exiting...");
        } else {
            println!("Server shut down successfully.");
        }
    } else if client {
        let debug = matches.is_present("debug");

        {
            profiling::scope!("Init logging");
            if !file_helper.game_path("logs/").exists() {
                info!("logs dir missing, creating it...");
                std::fs::create_dir_all(file_helper.game_path("logs/"))
                    .expect("Failed to create logs dir:");
            }

            CombinedLogger::init(vec![
                TermLogger::new(
                    if debug {
                        LevelFilter::Trace
                    } else {
                        LevelFilter::Info
                    },
                    ConfigBuilder::new()
                        .set_location_level(if debug {
                            LevelFilter::Error
                        } else {
                            LevelFilter::Off
                        })
                        .set_level_padding(simplelog::LevelPadding::Right)
                        .set_target_level(LevelFilter::Off)
                        .set_time_offset_to_local()
                        .unwrap()
                        .build(),
                    TerminalMode::Mixed,
                    simplelog::ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Trace,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .set_level_padding(simplelog::LevelPadding::Right)
                        .set_target_level(LevelFilter::Off)
                        .set_time_offset_to_local()
                        .unwrap()
                        .build(),
                    File::create(file_helper.game_path("logs/client_latest.log")).unwrap(),
                ),
            ])
            .unwrap();
        }

        std::panic::set_hook(Box::new(|info| {
            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");

            error!("thread '{}' {}", name, info);
            error!(
                "See client_latest.log for more details. Backtrace:\n{:?}",
                Backtrace::new()
            );
        }));

        info!("Starting client...");

        info!("Starting init...");

        let event_loop = {
            profiling::scope!("EventLoop::new");
            glutin::event_loop::EventLoop::new()
        };
        let r = Renderer::create(&event_loop, &file_helper).expect("Renderer::create failed"); // want to panic

        info!("Finished init.");

        let mut game: ClientGame = ClientGame::new(file_helper);

        if let Some(w) = &mut game.data.world {
            let rigid_body = RigidBodyBuilder::new_dynamic()
                .position(Isometry2::new(Vector2::new(0.0, 20.0), 0.0))
                .lock_rotations()
                .gravity_scale(0.0)
                .build();
            let handle = w.physics.bodies.insert(rigid_body);
            let collider =
                ColliderBuilder::cuboid(12.0 / PHYSICS_SCALE / 2.0, 20.0 / PHYSICS_SCALE / 2.0)
                    .collision_groups(InteractionGroups::new(
                        CollisionFlags::PLAYER.bits(),
                        (CollisionFlags::RIGIDBODY | CollisionFlags::ENTITY).bits(),
                    ))
                    .density(1.5)
                    .friction(0.3)
                    .build();
            let co_handle =
                w.physics
                    .colliders
                    .insert_with_parent(collider, handle, &mut w.physics.bodies);
            let bo_handle = w
                .physics
                .fluid_pipeline
                .liquid_world
                .add_boundary(Boundary::new(Vec::new()));
            w.physics.fluid_pipeline.coupling.register_coupling(
                bo_handle,
                co_handle,
                ColliderSampling::DynamicContactSampling,
            );

            let player = w
                .ecs
                .create_entity()
                .with(Player {
                    movement: PlayerMovementMode::Free,
                    clipboard: PlayerClipboard::default(),
                })
                .with(GameEntity)
                .with(PhysicsEntity {
                    on_ground: false,
                    gravity: 0.5,
                    edge_clip_distance: 2.0,
                    collision: true,
                    collide_with_sand: true,
                })
                .with(Persistent)
                .with(Position { x: 0.0, y: -20.0 })
                .with(Velocity { x: 0.0, y: 0.0 })
                .with(Hitbox { x1: -6.0, y1: -10.0, x2: 6.0, y2: 10.0 })
                .with(Loader)
                .with(RigidBodyComponent::of(handle))
                .build();

            let _camera = w
                .ecs
                .create_entity()
                .with(Camera)
                .with(Position { x: 0.0, y: 0.0 })
                .with(Velocity { x: 0.0, y: 0.0 })
                .with(AutoTarget {
                    target: Target::Entity(player),
                    offset: (0.0, 0.0),
                    style: TargetStyle::Locked,
                })
                .build();

            game.client.world = Some(ClientWorld { local_entity: Some(player) });
        };

        info!("Starting main loop...");
        game.run(r, matches, event_loop);
        info!("Goodbye!");
    }

    Ok(())
}
